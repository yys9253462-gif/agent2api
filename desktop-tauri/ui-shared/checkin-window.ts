export const DEFAULT_CHECKIN_START = '00:01'
export const DEFAULT_CHECKIN_END = '00:10'
export const DEFAULT_CHECKIN_SPREAD_MINUTES = 9

export function checkinTimeMinutes(time: string): number | null {
  if (!/^(?:[01]\d|2[0-3]):[0-5]\d$/.test(time)) return null
  return Number(time.slice(0, 2)) * 60 + Number(time.slice(3))
}

function formatMinutes(value: number): string {
  return `${String(Math.floor(value / 60)).padStart(2, '0')}:${String(value % 60).padStart(2, '0')}`
}

export function defaultCheckinEnd(start: string): string {
  const minutes = checkinTimeMinutes(start)
  return minutes === null ? DEFAULT_CHECKIN_END : formatMinutes(Math.min(1439, minutes + DEFAULT_CHECKIN_SPREAD_MINUTES))
}

export function checkinEnd(start: string, end?: string): string {
  return validCheckinRange(start, end ?? '') ? end! : defaultCheckinEnd(start)
}

export function validCheckinRange(start: string, end: string): boolean {
  const from = checkinTimeMinutes(start)
  const to = checkinTimeMinutes(end)
  return from !== null && to !== null && to >= from
}

export function providerCheckinEnd(start: string, defaultStart: string, defaultEnd?: string): string {
  const from = checkinTimeMinutes(start)
  const base = checkinTimeMinutes(defaultStart)
  const end = checkinTimeMinutes(checkinEnd(defaultStart, defaultEnd))
  if (from === null || base === null || end === null) return start
  return formatMinutes(Math.min(1439, from + end - base))
}

export function checkinRangeText(start: string, end?: string): string {
  return end && end !== start ? `${start}–${end}` : start
}

export function checkinMakeupTime(end: string, retryTime: string, gapMinutes: number): string {
  const main = checkinTimeMinutes(end)
  const retry = checkinTimeMinutes(retryTime)
  if (main === null || retry === null) return ''
  const target = Math.max(retry, main + Math.max(1, gapMinutes))
  return target < 1440 ? formatMinutes(target) : ''
}
