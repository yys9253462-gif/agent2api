import { t } from '../i18n'
import type { AutoState, ScheduleEntry } from './checkin-types'
import type { ScheduleProviderRow } from '../../../ui-shared/checkin'
import { checkinEnd, checkinMakeupTime, checkinRangeText, providerCheckinEnd } from '../../../ui-shared/checkin-window'

export { checkinEnd, checkinRangeText, providerCheckinEnd } from '../../../ui-shared/checkin-window'

export type ScheduleOption = {
  id: string
  label: string
  builtinTime?: string | null
  available: boolean
}

export type ScheduleTimeGroup = { time: string; timeEnd?: string; rows: ScheduleProviderRow[] }

export function scheduleOptions(auto: AutoState): ScheduleOption[] {
  const options = new Map<string, ScheduleOption>(
    (auto.providerOptions ?? []).map(option => [option.id, { ...option, available: true }]),
  )
  for (const entry of auto.schedules ?? []) {
    for (const id of entry.providers) {
      if (options.has(id)) continue
      const row = entry.providerRows?.find(item => item.id === id)
      options.set(id, { id, label: row?.label || id, builtinTime: row?.builtin, available: false })
    }
  }
  return [...options.values()]
}

export function effectiveScheduleTime(
  time: string, providerTimes: Record<string, string>, option: ScheduleOption,
): string {
  return providerTimes[option.id] || option.builtinTime || time
}

export function scheduleRows(entry: ScheduleEntry, options: ScheduleOption[]): ScheduleProviderRow[] {
  return entry.providers.map(id => {
    const row = entry.providerRows?.find(item => item.id === id)
    if (row) return row
    const option: ScheduleOption = options.find(item => item.id === id) ?? { id, label: id, available: false }
    return {
      id, label: option.label, time: effectiveScheduleTime(entry.time, entry.providerTimes ?? {}, option),
      timeEnd: providerCheckinEnd(effectiveScheduleTime(entry.time, entry.providerTimes ?? {}, option), entry.time, entry.timeEnd),
      builtin: option.builtinTime ?? null, firedToday: entry.lastFiredToday,
    }
  })
}

export function groupScheduleRows(rows: ScheduleProviderRow[]): ScheduleTimeGroup[] {
  const groups = new Map<string, ScheduleProviderRow[]>()
  for (const row of rows) {
    const key = checkinRangeText(row.time, row.timeEnd)
    groups.set(key, [...(groups.get(key) ?? []), row])
  }
  return [...groups].sort(([a], [b]) => a.localeCompare(b)).map(([, rows]) => ({
    time: rows[0].time, timeEnd: rows[0].timeEnd, rows,
  }))
}

export function scheduleGroupLabel(group: ScheduleTimeGroup, entry: ScheduleEntry): string {
  if (group.rows.every(row => !row.builtin && !entry.providerTimes?.[row.id])) return t('跟随默认时间')
  if (group.rows.every(row => row.builtin === row.time && !entry.providerTimes?.[row.id])) return t('平台专属时间')
  return t('自定义时间')
}

export function schedulePayload(entry: ScheduleEntry, changes: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: entry.id, label: entry.label ?? '', enabled: entry.enabled,
    time: entry.time, timeEnd: checkinEnd(entry.time, entry.timeEnd), providers: [...entry.providers],
    providerTimes: { ...entry.providerTimes }, retryTime: entry.retryTime,
    retryGapMinutes: entry.retryGapMinutes, retryLimit: entry.retryLimit, ...changes,
  }
}

export function validScheduleTime(time: string): boolean {
  return /^(?:[01]\d|2[0-3]):[0-5]\d$/.test(time)
}

export function previewMakeupTime(time: string, retryTime: string, gapMinutes: number): string {
  return checkinMakeupTime(time, retryTime, gapMinutes)
}

export function makeupScheduleRows(entry: ScheduleEntry, options: ScheduleOption[]): ScheduleProviderRow[] {
  return scheduleRows(entry, options).map(row => ({
    ...row,
    time: entry.providerRetryTimes?.[row.id]
      || previewMakeupTime(row.timeEnd || row.time, entry.retryTime, entry.retryGapMinutes),
    timeEnd: undefined,
  })).filter(row => row.time)
}

export function activityRetryEnabled(entry: ScheduleEntry): boolean {
  return entry.retryGapMinutes > 0 && entry.retryLimit > 0
}

export function beijingScheduleTime(value: unknown): string {
  const stamp = Number(value)
  if (!Number.isFinite(stamp) || stamp <= 0) return '—'
  return new Intl.DateTimeFormat('zh-CN', {
    timeZone: 'Asia/Shanghai', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit',
    hourCycle: 'h23',
  }).format(stamp)
}

export function scheduleName(entry: ScheduleEntry): string {
  return entry.displayLabel || entry.label || entry.time || entry.id
}

export function goCheckinTab(tab: 'schedule' | 'makeup' | 'records'): void {
  window.dispatchEvent(new CustomEvent('wb:checkin-tab', { detail: { tab } }))
}
