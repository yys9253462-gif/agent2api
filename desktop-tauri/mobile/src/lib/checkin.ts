import type { ActivityResult, CheckinRun } from '../api/checkin'
import { beijingDayKey } from './format'
import { t } from '../i18n'
import { rowCompleted, rowPending, rowMessage } from '../../../ui-shared/checkin-result'

export function checkinSummary(result: CheckinRun, single = false): { text: string; tone: 'ok' | 'err' | 'warn' } {
  const rows = result.results ?? []
  if (single && rows.length === 1) return activitySummary(rows[0])
  const failedRows = rows.filter(row => !rowCompleted(row) && !rowPending(row))
  const failed = result.failedCount ?? (Array.isArray(result.failed) ? result.failed.length : result.failed) ?? failedRows.length
  const text = t('签到完成：{success}/{total} 成功，保活 {active}，失败 {failed}', {
    success: Number(result.succeeded) || 0,
    total: Number(result.total) || rows.length,
    active: Number(result.active) || 0,
    failed,
  })
  const failures = Array.isArray(result.failed) ? result.failed : failedRows.map(rowMessage).filter(Boolean)
  return {
    text: failures.length > 0 ? `${text}（${failures.slice(0, 5).join('；')}）` : text,
    tone: failed > 0 ? 'err' : Number(result.succeeded) > 0 || Number(result.active) > 0 ? 'ok' : 'warn',
  }
}

export function activitySummary(result: ActivityResult): { text: string; tone: 'ok' | 'err' | 'warn' } {
  const completed = rowCompleted(result)
  return {
    text: rowMessage(result) || (completed ? t('执行完成') : t('未完成，请查看签到记录')),
    tone: completed ? 'ok' : rowPending(result) ? 'warn' : 'err',
  }
}

export function checkinModeLabel(mode: string): string {
  switch (mode) {
    case 'checkin': return t('签到')
    case 'keepalive': return t('保活')
    case 'claim': return t('领取')
    case 'full': return t('保活 + 领取')
    default: return mode
  }
}

export function claimedPlansToday(ledger: Record<string, unknown> | null | undefined): string[] {
  if (!ledger || typeof ledger !== 'object' || Array.isArray(ledger)) return []
  const today = beijingDayKey()
  return Object.entries(ledger).filter(([, raw]) => {
    const at = Number(raw)
    return Number.isFinite(at) && at > 0 && at < 8.64e15 - 8 * 3_600_000 && beijingDayKey(at) === today
  }).map(([id]) => id)
}

export function recordRangeStart(range: string): number | undefined {
  if (range === 'all') return undefined
  const beijing = new Date(Date.now() + 8 * 3_600_000)
  const midnight = Date.UTC(beijing.getUTCFullYear(), beijing.getUTCMonth(), beijing.getUTCDate()) - 8 * 3_600_000
  if (range === 'today') return midnight
  if (range === 'month') return Date.UTC(beijing.getUTCFullYear(), beijing.getUTCMonth(), 1) - 8 * 3_600_000
  const days = Number(range)
  return Number.isFinite(days) && days > 0 ? midnight - (days - 1) * 86_400_000 : undefined
}
