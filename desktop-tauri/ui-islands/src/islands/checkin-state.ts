/**
 * 签到中心的**状态 store**（非岛：`.ts` 不被 import.meta.glob 当岛加载）。
 *
 * ── 数据分两层 ──────────────────────────────────────────────
 *   · 快照层 —— `GET /api/checkin-center` 的一次聚合：每日签到分组（判定在后端，
 *     与批量签到同一对判据）、自动签到设置、签到历史台账、一次性项的账号清单。
 *     页面打开 / 切入时拉一次，签到动作完成后整体重拉。
 *   · 惰性层 —— 新手任务（Loomy）的任务状态。它是上游查询，快照刻意不带
 *     （见 api::checkin_center 的模块头）；这里按账号缓存查询结果，
 *     界面用「上次查询时间」如实呈现缓存的新旧。
 *
 * ── 为什么是模块级 store + useSyncExternalStore ──────────────
 * 对外契约（`wbCheckinPanel.load()`）从 React 之外调用（app.js 切页钩子），
 * 必须与界面共用同一份状态 —— 组件内部的 useState 做不到。与 accounts-store
 * 同一模式：状态是一份模块级快照，改动一律走 patch()（换新对象再通知）。
 */

import {
  errorMessage, formatTime, shared, toast,
  type OnboardingTask, type OnboardingTaskRaw,
} from './accounts-shared'

/* ─── 快照类型（/api/checkin-center 的响应，字段可能缺，逐个归一）─── */

export type CheckinAccountRow = {
  id: string
  name: string
  available: boolean
  checkinAt: number | null
  checkedInToday: boolean
}

export type CheckinProviderGroup = {
  id: string
  label: string
  accounts: CheckinAccountRow[]
  doneCount: number
  totalCount: number
}

export type OutOfScopeGroup = { label: string; reason: string; count: number }

export type CheckinHistoryEntry = {
  at?: number | null
  date?: string | null
  reason?: string | null
  succeeded?: number | null
  total?: number | null
  skipped?: number | null
  failed?: string[] | null
  failedCount?: number | null
}

/** 自动签到的状态（与 /api/auto-checkin 同形；tasks-panel 迁来同款读法） */
export type AutoCheckinState = {
  enabled?: boolean
  time?: string
  providers?: string[]
  providerOptions?: Array<{ id: string; label: string }>
  lastFiredToday?: boolean
  nextRunAt?: number | null
  lastResult?: CheckinHistoryEntry | null
  running?: boolean
}

export type CheckinCenterSnapshot = {
  daily: {
    providers: CheckinProviderGroup[]
    outOfScope: OutOfScopeGroup[]
    todayDone: number
    todayEligible: number
  }
  extras: {
    onboarding: Array<{ id: string; name: string }>
    welfare: Array<{ id: string; name: string }>
    plans: Array<{ id: string; name: string; claimAt?: number | null; claimPlans?: Record<string, number> | null }>
  }
  auto: AutoCheckinState
  history: CheckinHistoryEntry[]
}

/** 一个 Loomy 账号的 novice 任务缓存（status: idle = 还没查过） */
export type OnboardingCache = {
  status: 'idle' | 'loading' | 'loaded' | 'error'
  error?: string
  checkedAt?: number
  tasks: OnboardingTask[]
  unclaimed: number
  earned: number
  total: number
  claiming: boolean
}

/* ─── 本岛用到的桥（在 accounts-shared 的 AccountsBridge 之上补签到两组）─── */

type CheckinBridge = {
  getCheckinCenter?(): Promise<CheckinCenterSnapshot | null | undefined>
  getAutoCheckin?(): Promise<AutoCheckinState>
  saveAutoCheckin?(patch: Record<string, unknown>): Promise<AutoCheckinState>
  runAutoCheckinNow?(): Promise<(CheckinHistoryEntry & AutoCheckinState) | null | undefined>
  checkinAllAccounts?(id?: string | null): Promise<{
    results?: Array<Record<string, unknown>>
    succeeded?: number
    total?: number
  } | null | undefined>
  getOnboardingTasks?(id: string): Promise<{
    tasks?: OnboardingTaskRaw[]
    earned?: unknown
    total?: unknown
    unclaimed?: unknown
  } | null | undefined>
  claimOnboardingTasks?(id: string): Promise<{
    results?: Array<Record<string, unknown>>
    tasks?: OnboardingTaskRaw[]
    earned?: unknown
    total?: unknown
    unclaimed?: unknown
  } | null | undefined>
}

function bridge(): CheckinBridge {
  return (shared().workbuddyDesktop ?? {}) as CheckinBridge
}

/**
 * 账号页视图桥的窄读（wbAccountsView 由 accounts-page 自挂，不在 accounts-shared
 * 的 SharedWindow 声明里）。这里只取「签到后刷新余额读数」这一个动作。
 */
function accountsView(): { wbAccountsView?: { refreshUsageAfterCheckin?: (id?: string | null) => unknown } } {
  return window as unknown as { wbAccountsView?: { refreshUsageAfterCheckin?: (id?: string | null) => unknown } }
}

/* ─── store ─────────────────────────────────── */

export type CheckinStore = {
  /** 每次变更 +1；React 靠它判断「该重画了」（快照对象同时被换掉） */
  version: number
  /** 聚合快照（null = 还没读到 / 读失败） */
  snapshot: CheckinCenterSnapshot | null
  /** 快照至少成功拉过一次（失败后的重试交给用户手点刷新，不自动轮询） */
  loaded: boolean
  loadError: string
  /** 展开的每日签到分组（按提供商 id） */
  expanded: ReadonlySet<string>
  /** 「立即全部签到」在途（防重 + 按钮文案） */
  runningAll: boolean
  /** 自动签到设置保存中（开关 / 时刻 / 范围控件临时禁用） */
  autoSaving: boolean
  /** 签到时刻的编辑草稿：null = 跟随后端值 */
  autoTimeDraft: string | null
  /** 单账号签到在途（行内按钮的去重闸） */
  signing: ReadonlySet<string>
  /** Loomy 新手任务的按账号缓存 */
  onboarding: ReadonlyMap<string, OnboardingCache>
}

let store: CheckinStore = {
  version: 0,
  snapshot: null,
  loaded: false,
  loadError: '',
  expanded: new Set<string>(),
  runningAll: false,
  autoSaving: false,
  autoTimeDraft: null,
  signing: new Set<string>(),
  onboarding: new Map<string, OnboardingCache>(),
}

const subscribers = new Set<() => void>()

function emitChange(): void {
  for (const notify of subscribers) notify()
}

/** 换新对象再通知（与 accounts-store 的 patch 同一口径） */
function patch(next: Partial<CheckinStore>): void {
  store = { ...store, ...next, version: store.version + 1 }
  emitChange()
}

export function getCheckinStore(): CheckinStore {
  return store
}

export function subscribeCheckinStore(notify: () => void): () => void {
  subscribers.add(notify)
  return () => subscribers.delete(notify)
}

/* ─── 归一（后端字段可能缺，读的时候逐个兜底）─── */

function normalizeTask(raw: OnboardingTaskRaw): OnboardingTask | null {
  const key = typeof raw?.key === 'string' ? raw.key : ''
  if (!key) return null
  return {
    key,
    title: typeof raw.title === 'string' && raw.title ? raw.title : key,
    group: typeof raw.group === 'string' && raw.group ? raw.group : '',
    points: Number(raw.points) || 0,
    done: raw.done === true,
  }
}

function normalizeTasks(raw: unknown): OnboardingTask[] {
  const list = Array.isArray(raw) ? raw : []
  return list.map(normalizeTask).filter((task): task is OnboardingTask => task !== null)
}

function numberOf(value: unknown): number {
  return Number(value) || 0
}

/* ─── 动作 ─────────────────────────────────── */

/**
 * 拉聚合快照。签到动作完成后也走这里重拉（签到会改 checkinAt / 台账 /
 * 自动签到状态，一份接口全兜住）。失败不清空已有快照 —— 老数据还在展示，
 * 错误提示挂在页面头。
 */
export async function loadCheckinCenter(): Promise<void> {
  try {
    const data = await bridge().getCheckinCenter?.()
    if (!data) throw new Error('后端未返回签到中心数据')
    patch({
      snapshot: data,
      loaded: true,
      loadError: '',
      // 时刻草稿跟着后端值走（首次加载 / 异地修改后都以服务端为准）
      autoTimeDraft: null,
    })
  } catch (error) {
    patch({ loaded: true, loadError: errorMessage(error) })
  }
}

/** 展开 / 收起一个提供商分组 */
export function toggleProviderExpand(id: string): void {
  const next = new Set(store.expanded)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  patch({ expanded: next })
}

/**
 * 展开 / 收起一个账号的新手任务清单。
 *
 * 键加 `onb:` 前缀与每日签到的提供商分组共用同一个 `expanded` 集合（两边的
 * 交互语义一致，没必要开第二份状态）；账号 id 与提供商 id 不会撞（前者是
 * 落盘的主键），前缀是给读代码的人的——一眼看出这颗键属于哪张卡。
 */
export function toggleOnboardingExpand(id: string): void {
  const key = `onb:${id}`
  const next = new Set(store.expanded)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  patch({ expanded: next })
}

/** 某账号的新手任务清单当前是否展开 */
export function onboardingExpanded(id: string): boolean {
  return store.expanded.has(`onb:${id}`)
}

/** 「立即全部签到」：走 POST /api/auto-checkin/run（与定时同一条执行体，自带防重） */
export async function runAllCheckin(): Promise<void> {
  if (store.runningAll) return
  patch({ runningAll: true })
  try {
    const result = await bridge().runAutoCheckinNow?.()
    if (result) {
      const succeeded = numberOf(result.succeeded)
      const total = numberOf(result.total)
      const failed = numberOf(result.failedCount)
      if (failed > 0) {
        toast(`签到完成：${succeeded}/${total} 成功，${failed} 个失败`, 'err')
      } else {
        toast(`✅ 签到完成：${succeeded}/${total} 个账号成功领取`)
      }
      // 余额读数是账号页自己缓存里的，不查它还是签到前的旧值（不 await：
      // 那是账号页的动作层，静默刷新，结果落在余额列上）
      accountsView().wbAccountsView?.refreshUsageAfterCheckin?.(null)
    }
  } catch (error) {
    toast(`签到失败：${errorMessage(error)}`, 'err')
  } finally {
    patch({ runningAll: false })
    await loadCheckinCenter()
    // 快照到位后自动处理 Loomy 新手任务（不 await：签到结果已经播报过，
    // 查询与领取是后台的一次跟进，进度直接落在「待领新手任务」卡上）
    void autoProcessOnboarding(getCheckinStore().snapshot?.extras.onboarding ?? [])
    shared().wbApp?.refresh?.()
  }
}

/**
 * 单账号签到（签到中心明细行上的「签到 / 重签」；端点与原账号页行内按钮相同）。
 */
export async function signSingleAccount(id: string): Promise<void> {
  if (store.signing.has(id)) return
  const next = new Set(store.signing)
  next.add(id)
  patch({ signing: next })
  try {
    const result = await bridge().checkinAllAccounts?.(id)
    const row = result?.results?.[0]
    const claim = row?.claim as { success?: boolean; msg?: string } | undefined
    if (row?.error) {
      toast(`签到失败：${row.error}`, 'err')
    } else if (claim?.success === true) {
      toast(`✅ ${claim.msg || '签到成功'}`)
    } else {
      toast(claim?.msg || '今日已领取', 'err')
    }
    accountsView().wbAccountsView?.refreshUsageAfterCheckin?.(id)
  } catch (error) {
    toast(`签到失败：${errorMessage(error)}`, 'err')
  } finally {
    const rest = new Set(getCheckinStore().signing)
    rest.delete(id)
    patch({ signing: rest })
    await loadCheckinCenter()
    // 只处理刚签的这个账号：从快照的 Loomy 清单里过滤，不是 Loomy 账号则数组为空
    // （新手任务目前只有 Loomy 一家有，别的家不发无意义的查询）
    const loomyRows = (getCheckinStore().snapshot?.extras.onboarding ?? [])
      .filter(row => row.id === id)
    void autoProcessOnboarding(loomyRows)
    shared().wbApp?.refresh?.()
  }
}

/* ─── 自动签到设置（自 tasks-panel 迁入，提交语义保持一致）─── */

/**
 * 保存签到设置（开关 / 时刻 / 范围共用）。保存期间控件全部禁用；开关一起
 * 提交当前时刻（与旧实现同款：带上未提交的编辑草稿）。
 */
export async function saveAutoCheckin(patchBody: Record<string, unknown>, label: string): Promise<void> {
  if (store.autoSaving) return
  patch({ autoSaving: true })
  try {
    const next = await bridge().saveAutoCheckin?.(patchBody)
    if (next) {
      patch({ snapshot: mergeAuto(next), autoTimeDraft: null })
      toast(`已保存：${label}`)
    }
  } catch (error) {
    toast(`保存失败：${errorMessage(error)}`, 'err')
  } finally {
    patch({ autoSaving: false })
  }
}

/** 签到时刻的编辑草稿（输入过程；提交在 blur / 回车时走 submitAutoTime） */
export function setAutoTimeDraft(value: string): void {
  patch({ autoTimeDraft: value })
}

/** 签到时刻提交（失焦 / 回车）：格式由后端校验，错误原样吐回 */
export async function submitAutoTime(value: string): Promise<void> {
  const time = value.trim()
  if (!time) return
  await saveAutoCheckin(
    { enabled: getCheckinStore().snapshot?.auto?.enabled === true, time },
    '签到触发时刻',
  )
}

/** 勾选 / 取消一家签到提供商（提交完整清单，后端校验至少一家） */
export async function toggleAutoProvider(optionId: string, checked: boolean): Promise<void> {
  const snapshot = getCheckinStore().snapshot
  const options = snapshot?.auto?.providerOptions ?? []
  const picked = new Set(snapshot?.auto?.providers ?? [])
  if (checked) picked.add(optionId)
  else picked.delete(optionId)
  // 顺序取 providerOptions 的注册顺序（与后端落盘口径一致）
  const providers = options.map(option => option.id).filter(id => picked.has(id))
  await saveAutoCheckin({ providers }, '签到提供商')
}

/** 快照里只换 auto 段（保存响应是新状态，其余快照原样保留） */
function mergeAuto(next: AutoCheckinState): CheckinCenterSnapshot | null {
  const current = getCheckinStore().snapshot
  if (!current) return current
  return { ...current, auto: next }
}

/* ─── Loomy 新手任务（惰性查询 + 一键领取）─── */

/**
 * 签到完成后的自动处理（原账号页「签到后自动弹窗领取」口径的延续）：
 * 对 [`rows`] 里的 Loomy 账号逐个查询任务状态（只读），有未领取的**立即自动领取**。
 *
 * ── 为什么自动领取是安全的 ──────────────────────────────────
 * 新手任务是一次性福利，服务端幂等（重复上报 alreadyCompleted，不重复加分），
 * 自动领取不会多拿；全部领完后查询结果 unclaimed=0，之后签到就只是签到。
 * 逐账号串行（与签到同一条防风控口径），单账号查询失败不拖累其它账号。
 *
 * `rows` 传快照的 `extras.onboarding`（全部 Loomy 账号）；单账号签到时传只含
 * 该账号的数组（用户点的是谁就处理谁）。领取完成后 claimOnboarding 内部会
 * 重拉快照，「待领新手任务」总览卡随之归零。
 */
async function autoProcessOnboarding(rows: Array<{ id: string }>): Promise<void> {
  for (const row of rows) {
    await queryOnboarding(row.id)
  }
  for (const row of rows) {
    const cache = getCheckinStore().onboarding.get(row.id)
    if (cache?.status === 'loaded' && cache.unclaimed > 0 && !cache.claiming) {
      await claimOnboarding(row.id)
    }
  }
}

function onboardingOf(id: string): OnboardingCache {
  return store.onboarding.get(id) ?? {
    status: 'idle', tasks: [], unclaimed: 0, earned: 0, total: 0, claiming: false,
  }
}

function patchOnboarding(id: string, next: Partial<OnboardingCache>): void {
  const map = new Map(store.onboarding)
  map.set(id, { ...onboardingOf(id), ...next })
  patch({ onboarding: map })
}

/**
 * 查询一个账号的任务状态（不领，只读）。
 *
 * `expand: true`（界面上点「查询任务」）在查到任务后**默认展开**清单 ——
 * 查询这个动作本身就是「我要看」，让它多付一次点击才能看到结果是反的；
 * 签到后的自动处理不展开（autoProcessOnboarding），不让后台动作替用户动界面。
 */
export async function queryOnboarding(id: string, options?: { expand?: boolean }): Promise<void> {
  const current = onboardingOf(id)
  if (current.status === 'loading' || current.claiming) return
  patchOnboarding(id, { status: 'loading', error: undefined })
  try {
    const data = await bridge().getOnboardingTasks?.(id)
    const tasks = normalizeTasks(data?.tasks)
    patchOnboarding(id, {
      status: 'loaded',
      checkedAt: Date.now(),
      tasks,
      unclaimed: tasks.filter(task => !task.done).length,
      earned: numberOf(data?.earned),
      total: numberOf(data?.total),
    })
    if (options?.expand === true && tasks.length > 0 && !onboardingExpanded(id)) {
      toggleOnboardingExpand(id)
    }
  } catch (error) {
    patchOnboarding(id, { status: 'error', error: errorMessage(error) })
  }
}

/**
 * 一键领取：串行上报全部未完成的 key（服务端幂等，重复点不会重复加分）。
 * 行级状态实时推进；完成后重拉快照 —— 任务状态本身不在快照里，
 * 重拉是为了让时间线 / 总览跟上一轮刚发生的动作。
 */
export async function claimOnboarding(id: string): Promise<void> {
  const current = onboardingOf(id)
  if (current.claiming || current.status !== 'loaded') return
  patchOnboarding(id, { claiming: true })
  try {
    // 行级「领取中」：失败过的行也重试（服务端幂等，重试是唯一出路）
    patchOnboarding(id, {
      tasks: onboardingOf(id).tasks.map(task =>
        task.done ? task : { ...task, claiming: true, error: undefined },
      ),
    })
    const data = await bridge().claimOnboardingTasks?.(id)
    const failedKeys = new Map<string, string>()
    const rows = Array.isArray(data?.results) ? data.results : []
    for (const row of rows) {
      const key = typeof row?.key === 'string' ? row.key : ''
      if (key && row?.ok !== true) failedKeys.set(key, String(row?.error || '领取失败'))
    }
    const server = normalizeTasks(data?.tasks)
    const merged = (server.length ? server : onboardingOf(id).tasks).map(task => ({
      ...task,
      claiming: false,
      error: failedKeys.get(task.key),
    }))
    const claimedCount = merged.filter(task => task.done && !failedKeys.has(task.key)).length
    patchOnboarding(id, {
      claiming: false,
      checkedAt: Date.now(),
      tasks: merged,
      unclaimed: merged.filter(task => !task.done).length,
      earned: numberOf(data?.earned),
      total: numberOf(data?.total),
    })
    const failed = merged.filter(task => task.error && !task.done).length
    if (failed) {
      toast(`新手任务有 ${failed} 项领取失败，可重试`, 'err')
    } else if (claimedCount) {
      toast(`✅ 新手任务领取完成，累计 ${numberOf(data?.earned)} 积分`)
    }
  } catch (error) {
    const message = errorMessage(error)
    patchOnboarding(id, {
      claiming: false,
      tasks: onboardingOf(id).tasks.map(task => ({
        ...task,
        claiming: false,
        error: task.done ? undefined : message,
      })),
    })
    toast(`领取失败：${message}`, 'err')
  } finally {
    await loadCheckinCenter()
  }
}

/* ─── 展示工具 ─────────────────────────────── */

/** 分组状态：全部已签 / 部分 / 全部待签 / 没有账号 */
export function groupTone(group: CheckinProviderGroup): 'done' | 'part' | 'todo' | 'none' {
  if (!group.totalCount) return 'none'
  if (group.doneCount >= group.totalCount) return 'done'
  if (group.doneCount > 0) return 'part'
  return 'todo'
}

/** 历史条目的一行摘要（时间线副标题） */
export function historyLine(entry: CheckinHistoryEntry): string {
  const succeeded = numberOf(entry.succeeded)
  const total = numberOf(entry.total)
  const skipped = numberOf(entry.skipped)
  const failed = numberOf(entry.failedCount)
  const parts = [`成功 ${succeeded}`]
  if (skipped > 0) parts.push(`跳过 ${skipped}`)
  if (failed > 0) parts.push(`失败 ${failed}`)
  return `${parts.join(' · ')}${total ? `（共 ${total} 个账号）` : ''}`
}

/** 「下次执行」的人话（today / tomorrow 相对面板时刻） */
export function nextRunText(auto: AutoCheckinState | null | undefined): string {
  if (auto?.enabled !== true) return '未开启'
  const at = Number(auto.nextRunAt)
  if (!Number.isFinite(at) || at <= 0) return formatTime(at) || `每天 ${auto.time}`
  return `${formatTime(at)}（每天 ${auto.time}）`
}
