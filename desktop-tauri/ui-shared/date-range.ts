export const DATE_RANGES = ['today', 'yesterday', '7', '30', 'month', 'all', 'custom'] as const
export type DateRange = typeof DATE_RANGES[number]
export type CustomDates = { start: string; end: string }
export type DateQuery = { startDate?: string; endDate?: string }
export const DAY_KEY_RE = /^\d{4}-\d{2}-\d{2}$/

export function dayKey(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

export function isDayKey(value: string): boolean {
  if (!DAY_KEY_RE.test(value)) return false
  const [year, month, day] = value.split('-').map(Number)
  const date = new Date(0)
  date.setFullYear(year, month - 1, day)
  date.setHours(0, 0, 0, 0)
  return dayKey(date) === value
}

export function defaultCustomRange(now = new Date()): CustomDates {
  return { start: dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 6)), end: dayKey(now) }
}

export function validCustomRange(value: unknown): CustomDates | null {
  if (!value || typeof value !== 'object') return null
  const { start, end } = value as { start?: unknown; end?: unknown }
  return typeof start === 'string' && typeof end === 'string' && isDayKey(start) && isDayKey(end) && start <= end
    ? { start, end } : null
}

export function readCustomRange(key: string): CustomDates {
  try {
    const raw = localStorage.getItem(key)
    if (raw) {
      const stored = validCustomRange(JSON.parse(raw))
      if (stored) return stored
    }
  } catch { /* 存储不可用时沿用近 7 天。 */ }
  return defaultCustomRange()
}

export function saveCustomRange(key: string, range: CustomDates): void {
  try { localStorage.setItem(key, JSON.stringify(range)) }
  catch { /* 存储不可用不影响当前筛选。 */ }
}

export function updateCustomRange<T extends CustomDates>(range: T, patch: Partial<CustomDates>): T {
  const next = { ...range, ...patch }
  if (!isDayKey(next.start) || !isDayKey(next.end)) return range
  if (next.start > next.end) next[patch.start !== undefined ? 'end' : 'start'] = patch.start ?? next.end
  return next
}

export function dateRangeQuery(range: string, custom: CustomDates, now = new Date()): DateQuery {
  if (range === 'all') return {}
  if (range === 'custom') return { startDate: custom.start, endDate: custom.end }
  const end = dayKey(now)
  if (range === 'yesterday') {
    const yesterday = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1))
    return { startDate: yesterday, endDate: yesterday }
  }
  if (range === 'today') return { startDate: end, endDate: end }
  if (range === 'month') return { startDate: dayKey(new Date(now.getFullYear(), now.getMonth(), 1)), endDate: end }
  if (range === '7' || range === '30') {
    return { startDate: dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - Number(range) + 1)), endDate: end }
  }
  return {}
}
