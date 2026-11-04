export type LogView = 'important' | 'all' | 'debug'
export type LogEntry = {
  id?: number
  ts?: number
  level?: string
  category?: string
  message?: string
  data?: Record<string, unknown> | null
}
export type LogGroup = {
  id: number
  firstTs: number
  lastTs: number
  count: number
  entry: LogEntry
  entries: LogEntry[]
}
export type LogPage = {
  entries?: LogEntry[]
  groups?: LogGroup[]
  total?: number
  matched?: number
  grouped?: number
  offset?: number
  limit?: number
  lastId?: number
  byLevel?: Record<string, number>
  categories?: Record<string, string>
  debugEnabled?: boolean
}
export type LogQuery = {
  view?: LogView
  aggregate?: boolean
  offset?: number
  limit?: number
  untilId?: number
  sinceId?: number
  level?: string
  category?: string
  keyword?: string
  start?: number
  end?: number
  startDate?: string
  endDate?: string
}

type Translate = (text: string) => string

export function logQueryString(query: LogQuery): string {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined && value !== '') params.set(key, typeof value === 'boolean' ? (value ? '1' : '0') : String(value))
  }
  return params.toString()
}

export function logGroups(page: LogPage | null | undefined): LogGroup[] {
  if (Array.isArray(page?.groups)) return page.groups
  return (page?.entries || []).map((entry, index) => ({
    id: entry.id ?? index, firstTs: entry.ts ?? 0, lastTs: entry.ts ?? 0,
    count: 1, entry, entries: [entry],
  }))
}

export function logText(entry: LogEntry, key: string): string {
  const value = entry.data?.[key]
  return typeof value === 'string' || typeof value === 'number' ? String(value) : ''
}

export function logLevel(level: string | undefined, t: Translate): string {
  switch (level) {
    case 'debug': return t('调试')
    case 'warn': return t('警告')
    case 'error': return t('错误')
    default: return t('信息')
  }
}

export function logCategory(category: string | undefined, t: Translate): string {
  switch (category) {
    case 'server': return t('服务')
    case 'auth': return t('登录')
    case 'account': return t('账号')
    case 'model': return t('模型')
    case 'upstream': return t('上游')
    case 'config': return t('配置')
    case 'checkin': return t('自动签到')
    case 'maintenance': return t('凭证自动维护')
    case 'update': return t('软件版本检查')
    case 'desensitize': return t('脱敏')
    case 'plugin': return t('插件')
    default: return category || '—'
  }
}

export function logSource(entry: LogEntry, t: Translate): string {
  return logText(entry, 'providerLabel') || logText(entry, 'provider')
    || logText(entry, 'component') || logCategory(entry.category, t)
}

export function logSummary(entry: LogEntry, t: Translate): string {
  const summary = logText(entry, 'summary') || (entry.message || '').replace(/^(?:\[[^\]]+\]\s*)+/, '')
  switch (summary) {
    case '计费接口请求': return t('计费接口请求')
    case '上游接口请求': return t('上游接口请求')
    case '出站参数归一': return t('出站参数归一')
    case '补充系统消息': return t('补充系统消息')
    case '余额查询完成': return t('余额查询完成')
    case '余额查询未配置': return t('余额查询未配置')
    case '余额查询失败': return t('余额查询失败')
    default: return summary || '—'
  }
}

export function logContext(entry: LogEntry): string {
  const parts = [logText(entry, 'accountName') || logText(entry, 'account'), logText(entry, 'model')]
  const from = logText(entry, 'from')
  const to = logText(entry, 'to')
  if (from || to) parts.push([from, to].filter(Boolean).join(' → '))
  const status = logText(entry, 'status')
  if (status) parts.push(`HTTP ${status}`)
  const detail = logText(entry, 'detail')
  if (detail && entry.level !== 'debug') parts.push(detail)
  return parts.filter(Boolean).join(' · ')
}

export function formatLogTime(ts: number | undefined, clockOnly = false): string {
  if (!ts) return '—'
  const date = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  const clock = `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
  if (clockOnly) return clock
  const day = `${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
  return date.getFullYear() === new Date().getFullYear() ? `${day} ${clock}` : `${date.getFullYear()}-${day} ${clock}`
}

export function groupTime(group: LogGroup): string {
  if (group.firstTs === group.lastTs) return formatLogTime(group.lastTs)
  const sameDay = new Date(group.firstTs).toDateString() === new Date(group.lastTs).toDateString()
  return `${formatLogTime(group.firstTs)}–${formatLogTime(group.lastTs, sameDay)}`
}

export function logRawText(entries: LogEntry[]): string {
  return entries.map(entry => JSON.stringify(entry)).join('\n')
}
