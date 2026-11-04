import type { QueryParams, ToolArgs, ToolResult, Translate } from './assistant-read-types'

export const PAGE_DEFAULT = 50
export const PAGE_MAX = 200
const PAGE_TEXT_MAX = 18000

export function object(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {}
}

export function rows(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter(item => item && typeof item === 'object' && !Array.isArray(item)) : []
}

export function number(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

export function text(value: unknown, max = 256): string {
  return typeof value === 'string' ? value.trim().slice(0, max) : ''
}

export function safeText(value: unknown, max = 800): string {
  if (typeof value !== 'string') return ''
  return value
    .replace(/(https?:\/\/[^\s?#]+)\?[^\s#]*/gi, '$1?[REDACTED]')
    .replace(/\b(authorization|proxy-authorization|cookie|set-cookie)\s*[:=]\s*[^\r\n]+/gi, '$1: [REDACTED]')
    .replace(/\b(Bearer|Basic)\s+[A-Za-z0-9+/_=.:-]+/gi, '$1 [REDACTED]')
    .replace(/(["']?(?:access[_-]?token|refresh[_-]?token|id[_-]?token|api[_-]?key|password|passwd|client[_-]?secret|session[_-]?(?:token|key)|token|secret|令牌|密码|密钥)["']?\s*[:=]\s*)(?:"[^"\r\n]*"|'[^'\r\n]*'|[^\s,;}&\]]+)/gi, '$1[REDACTED]')
    .replace(/\b(?:sk-[A-Za-z0-9_-]{12,}|ms-[A-Za-z0-9_-]{12,}|user_[A-Za-z0-9_-]{12,}|eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+)\b/g, '[REDACTED]')
    .replace(/\b(?:BDUSS|STOKEN|sid|session_id|ot_session|cc_[a-z_]+)=[^\s;,]+/gi, '[REDACTED]')
    .replace(/(https?:\/\/)[^\s/@]+:[^\s/@]+@/gi, '$1[REDACTED]@')
    .trim().slice(0, max)
}

export function diagnosticText(value: unknown, t: Translate, max = 800): string {
  const raw = typeof value === 'string' ? value : ''
  if (/首条消息非 system，已注入系统消息：|出站归一：|["'](?:messages|input|output|prompt|content|requestBody|responseBody|system_prompt)["']\s*:/i.test(raw)) {
    return t('原始报文或对话正文已省略')
  }
  return safeText(raw, max)
}

// 只允许选定字段上的标量；新增后端字段不会自动进入模型上下文。
export function fields(value: unknown, keys: readonly string[], max = 256): Record<string, unknown> {
  const source = object(value)
  const out: Record<string, unknown> = {}
  for (const key of keys) {
    const item = source[key]
    if (item === null || typeof item === 'boolean') out[key] = item
    else if (typeof item === 'number' && Number.isFinite(item)) out[key] = item
    else if (typeof item === 'string') out[key] = safeText(item, max)
  }
  return out
}

export function strings(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string').map(item => safeText(item, 256)) : []
}

export function expectObject(value: unknown, name: string, t: Translate): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(t('读取响应不完整：{name}', { name }))
  return value as Record<string, unknown>
}

export function expectRows(source: Record<string, unknown>, key: string, name: string, t: Translate): Record<string, unknown>[] {
  const value = source[key]
  if (!Array.isArray(value) || value.some(item => !item || typeof item !== 'object' || Array.isArray(item))) {
    throw new Error(t('读取响应不完整：{name}', { name }))
  }
  return value as Record<string, unknown>[]
}

export function enumArg(args: ToolArgs, key: string, allowed: readonly string[], fallback: string, t: Translate): string {
  if (args[key] === undefined) return fallback
  const value = text(args[key])
  if (!allowed.includes(value)) throw new Error(t('参数 {name} 只能是：{values}', { name: key, values: allowed.join(' / ') }))
  return value
}

export function stringArg(args: ToolArgs, key: string, t: Translate, max = 256): string {
  if (args[key] === undefined) return ''
  if (typeof args[key] !== 'string' || (args[key] as string).trim().length > max) {
    throw new Error(t('参数 {name} 必须是不超过 {max} 个字符的字符串', { name: key, max }))
  }
  return (args[key] as string).trim()
}

export function booleanArg(args: ToolArgs, key: string, t: Translate): boolean | undefined {
  if (args[key] === undefined) return undefined
  if (typeof args[key] !== 'boolean') throw new Error(t('参数 {name} 必须是布尔值', { name: key }))
  return args[key] as boolean
}

export function pageArgs(args: ToolArgs, t: Translate): { offset: number; limit: number } {
  const integer = (key: string, fallback: number, min: number) => {
    if (args[key] === undefined) return fallback
    const value = number(args[key])
    if (value === null || !Number.isSafeInteger(value) || value < min) {
      throw new Error(t('参数 {name} 必须是不小于 {min} 的整数', { name: key, min }))
    }
    return value
  }
  return { offset: integer('offset', 0, 0), limit: Math.min(PAGE_MAX, integer('limit', PAGE_DEFAULT, 1)) }
}

export function page<T>(items: T[], offset: number, limit: number, total = items.length, budget = PAGE_TEXT_MAX) {
  const out: T[] = []
  let size = 0
  for (const item of items.slice(0, limit)) {
    const length = JSON.stringify(item).length
    if (out.length && size + length > budget) break
    out.push(item)
    size += length
  }
  const nextOffset = offset + out.length
  const hasMore = nextOffset < total
  return {
    total, offset, limit, returned: out.length, hasMore,
    ...(hasMore ? { nextOffset } : {}),
    limitedBySize: out.length < Math.min(items.length, limit),
    items: out,
  }
}

export const PAGE_PARAMS = {
  offset: { type: 'integer', minimum: 0, description: '起点（从 0 开始）；翻页时传上一页的 nextOffset' },
  limit: { type: 'integer', minimum: 1, maximum: PAGE_MAX, description: `每页条数（默认 ${PAGE_DEFAULT}，最多 ${PAGE_MAX}）；较长结果可能缩小本页，请按返回的 nextOffset 翻页` },
}

export const TIME_PARAMS = {
  start: { type: 'number', description: '开始时间，Unix 毫秒时间戳（包含）；不填 = 不限' },
  end: { type: 'number', description: '结束时间，Unix 毫秒时间戳（不包含）；不填 = 本次查询时刻。翻页沿用响应中的 end，避免新日志挤动分页' },
}

export function timeQuery(args: ToolArgs, t: Translate): QueryParams {
  const out: QueryParams = {}
  for (const key of ['start', 'end']) {
    if (args[key] === undefined) continue
    const value = number(args[key])
    if (value === null || value < 0 || !Number.isSafeInteger(value)) throw new Error(t('参数 {name} 必须是毫秒时间戳', { name: key }))
    out[key] = value
  }
  if (out.end === undefined) out.end = Date.now() + 1
  if (typeof out.start === 'number' && out.start >= Number(out.end)) throw new Error(t('开始时间必须早于结束时间'))
  return out
}

export function result(payload: Record<string, unknown>, summary: string): ToolResult {
  return { text: JSON.stringify(payload), summary }
}

export function accountName(account: Record<string, unknown>): string {
  return text(account.displayName) || text(account.name) || text(account.nickname) || text(account.email) || text(account.id)
}

export function accountsOf(value: unknown, t: Translate) {
  const snapshot = expectObject(value, 'accounts', t)
  const accounts = expectRows(snapshot, 'accounts', 'accounts', t).slice()
  if (accounts.some(account => !text(account.id))) throw new Error(t('读取响应不完整：{name}', { name: 'accounts.id' }))
  accounts.sort((a, b) => (number(a.priority) ?? 0) - (number(b.priority) ?? 0))
  return { snapshot, accounts }
}

export function findAccount(accounts: Record<string, unknown>[], ref: string, provider: string, t: Translate) {
  if (!ref) throw new Error(t('没给账号名'))
  const scoped = provider ? accounts.filter(account => text(account.provider).toLowerCase() === provider.toLowerCase()) : accounts
  const byId = scoped.find(account => text(account.id) === ref)
  if (byId) return byId
  const named = scoped.filter(account => accountName(account) === ref)
  const matches = named.length ? named : scoped.filter(account => accountName(account).toLowerCase().includes(ref.toLowerCase()))
  if (matches.length === 1) return matches[0]
  if (!matches.length) throw new Error(t('没有找到账号「{name}」', { name: safeText(ref) }))
  throw new Error(t('账号「{name}」匹配到多条记录，请使用账号 ID：{ids}', {
    name: safeText(ref), ids: matches.slice(0, 6).map(account => safeText(account.id)).join('、'),
  }))
}

export function activeRateLimits(account: Record<string, unknown>, now: number, t: Translate) {
  return Object.entries(object(account.rateLimits))
    .filter(([, value]) => (number(object(value).resetAt) ?? 0) > now)
    .map(([model, value]) => ({
      model: safeText(model), ...fields(value, ['status', 'code']),
      resetAt: number(object(value).resetAt) as number,
      message: diagnosticText(object(value).message, t),
    }))
    .sort((a, b) => Number(a.resetAt) - Number(b.resetAt))
}

export function accountSummary(account: Record<string, unknown>, t: Translate, now: number) {
  return {
    ...fields(account, ['id', 'provider', 'priority', 'enabled', 'available', 'hasCredentials', 'chatSupported', 'needsRelogin']),
    name: safeText(accountName(account)),
    position: (number(account.priority) ?? 0) + 1,
    rateLimited: activeRateLimits(account, now, t).length > 0,
  }
}

export function accountMeta(snapshot: Record<string, unknown>) {
  const hidden = object(snapshot.hiddenAccounts)
  return {
    currentAccountId: snapshot.currentAccountId === null ? null : safeText(snapshot.currentAccountId),
    providers: rows(snapshot.providers).map(provider => fields(provider, ['id', 'label', 'count'])),
    hiddenAccounts: {
      total: number(hidden.total) ?? 0,
      providers: rows(hidden.providers).map(provider => fields(provider, ['id', 'label', 'count', 'reason'])),
    },
  }
}
