import { t } from '../../i18n'
import type { RequestEntry } from './request-cells'

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, ch => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[ch] ?? ch))
}

/* ─── 转发阶段（原 ui/request-phase.js，仍挂 window）──────────
 *
 * 同一套阶段要在**两处**渲染且必须逐字一致：列表的状态列（本文件）与详情弹窗
 * （islands/request-detail.tsx 的 StatusCell，它读 window.wbRequestPhase 的 badgeHtml /
 * elapsedLineHtml 并原样注入）。所以文案 / 类名 / 计时的唯一实现在这里，窗口接口照旧导出 ——
 * 不能并进列表渲染后就撤掉：那会让详情弹窗那格静默失去阶段显示（回落成「进行中」）。
 *
 * 数据来自后端在途期间写入的 `phase` / `phaseStartedAt`（见 core::upstream::usage::LogPhase）：
 * phase 空串 = 不在途（终态行、旧行），回落成通用的「进行中」。
 * 文案（含悬停说明）在模块求值时走 t()：岛加载晚于 head 里的 i18n 词典注入，拿得到译文。
 */
const PHASES: Record<string, { label: string; cls: string }> = {
  connecting: { label: t('连接中'), cls: 'phase-connecting' },
  waiting: { label: t('等待响应'), cls: 'phase-waiting' },
  streaming: { label: t('响应中'), cls: 'phase-streaming' },
  retrying: { label: t('重试中'), cls: 'phase-retrying' },
  queued: { label: t('排队中'), cls: 'phase-queued' },
}
const PHASE_FALLBACK_LABEL = t('进行中')
const PHASE_FALLBACK_CLS = 'running'
/** 每个阶段的悬停说明：徽章只有四个字，落点要说清「这一步在干什么」 */
const PHASE_TITLES: Record<string, string> = {
  connecting: t('请求已受理，正在选路、取凭证、建立上游连接'),
  waiting: t('上游请求已发出，正在等第一个字节到达（模型的思考时间也在这段）'),
  streaming: t('首帧已到，上游内容正在下发'),
  retrying: t('本轮尝试失败，正在退避等待或切换到下一个账号'),
  queued: t('上游模型繁忙，请求已排进上游队列；网关正按上游建议的时长等待后重发（不是登录态或额度问题）'),
  '': t('请求正在转发中，用时列显示的是已用时'),
}

/** 行对象里那个阶段字面量，非法值一律按「没有阶段」处理（不猜） */
export function phaseOf(entry: unknown): string {
  const raw = String((entry as RequestEntry | null)?.phase ?? '').trim().toLowerCase()
  return Object.prototype.hasOwnProperty.call(PHASES, raw) ? raw : ''
}

/** 阶段文案（空串阶段 → 「进行中」） */
export function phaseLabelOf(entry: unknown): string {
  const phase = phaseOf(entry)
  return phase ? PHASES[phase].label : PHASE_FALLBACK_LABEL
}

/**
 * 阶段耗时 → 展示文案（N秒 / N分M秒，与「用时」列同族）。
 *
 * 必须先排除空值再转数字：`Number(null)` **是 0**（不是 NaN），只判 Number.isFinite 会把
 * 「没有这个读数」的 null 当成「0 毫秒」，于是 max(1, …) 渲染出一个凭空编出来的「1秒」。
 * 拿不到读数返回空串，调用方据此整行省掉。
 */
export function phaseElapsedText(ms: unknown): string {
  if (ms === null || ms === undefined || ms === '') return ''
  const value = Number(ms)
  if (!Number.isFinite(value) || value < 0) return ''
  const seconds = Math.max(1, Math.floor(value / 1000))
  if (seconds < 60) return t('{n}秒', { n: seconds })
  return t('{m}分{s}秒', { m: Math.floor(seconds / 60), s: seconds % 60 })
}

/**
 * 当前阶段计时的读数（毫秒），取不到给 null。优先用服务端现算的 `phaseElapsedMs`（与库里的
 * 阶段起点同一时钟，浏览器时钟偏了也不会离谱）；老响应没有该字段时用 phaseStartedAt 本地减一次。
 */
function phaseElapsedMsOf(entry: unknown): number | null {
  const raw: unknown = (entry as RequestEntry | null)?.phaseElapsedMs
  if (raw !== null && raw !== undefined && raw !== '') {
    const server = Number(raw)
    if (Number.isFinite(server) && server >= 0) return server
  }
  const started = Number((entry as RequestEntry | null)?.phaseStartedAt)
  if (!Number.isFinite(started) || started <= 0) return null
  const local = Date.now() - started
  return Number.isFinite(local) ? Math.max(0, local) : null
}

/** 阶段徽章（呼吸点 + 文案）。只返回徽章本身，不含外层列容器 —— 列表与详情弹窗的容器不同 */
export function phaseBadgeHtml(entry: unknown): string {
  const phase = phaseOf(entry)
  const label = phase ? PHASES[phase].label : PHASE_FALLBACK_LABEL
  const cls = phase ? PHASES[phase].cls : PHASE_FALLBACK_CLS
  const title = PHASE_TITLES[phase] || PHASE_TITLES['']
  return `<span class="badge tag ${cls}" title="${escapeHtml(title)}">`
    + `<span class="req-live-dot" aria-hidden="true"></span>${escapeHtml(label)}</span>`
}

/** 阶段计时的第二行；**有值才渲染**（整行省掉而不是写占位：缺失的成因是「还没测到」） */
export function phaseElapsedLineHtml(entry: unknown): string {
  const text = phaseElapsedText(phaseElapsedMsOf(entry))
  if (!text) return ''
  const label = phaseLabelOf(entry)
  const tip = t('进入「{phase}」阶段已持续 {time}', { phase: label, time: text })
  return `<span class="req-phase-elapsed" title="${escapeHtml(tip)}">`
    + `${escapeHtml(text)}</span>`
}
