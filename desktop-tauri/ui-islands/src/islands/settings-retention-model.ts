import { t } from '../i18n'
import type { NumberField } from './settings-model'

export type RetentionField = NumberField & { kind: 'days' | 'count' }

export const RETENTION_MIN = 1
export const RETENTION_MAX = 99999
export const RETENTION_MIN_COUNT = 1
export const RETENTION_MAX_COUNT = 100_000_000

export const RETENTION_FIELDS: RetentionField[] = [
  {
    key: 'logRetentionDays',
    id: 'settings-retention-log',
    label: t('事件日志保留天数'),
    unit: t('天'),
    kind: 'days',
    min: RETENTION_MIN,
    max: RETENTION_MAX,
    hint: t('登录 / 账号切换 / 429 切换等系统事件，可填 1–99999 天。'),
  },
  {
    key: 'logRetentionCount',
    id: 'settings-retention-log-count',
    label: t('日志保留数量上限'),
    unit: t('条'),
    kind: 'count',
    min: RETENTION_MIN_COUNT,
    max: RETENTION_MAX_COUNT,
    hint: t('默认 10000 条，按日志级别分配保留容量，超出后清理最旧的记录；同时受保留天数限制。'),
  },
  {
    key: 'requestLogRetentionDays',
    id: 'settings-retention-request',
    label: t('请求日志保留天数'),
    unit: t('天'),
    kind: 'days',
    min: RETENTION_MIN,
    max: RETENTION_MAX,
    hint: t('「请求日志」页的逐条请求记录，默认 99999 天，可填 1–99999 天；同时受数量上限限制。'),
  },
  {
    key: 'requestLogRetentionCount',
    id: 'settings-retention-request-count',
    label: t('请求日志保留数量上限'),
    unit: t('条'),
    kind: 'count',
    min: RETENTION_MIN_COUNT,
    max: RETENTION_MAX_COUNT,
    hint: t('默认 1000000 条（100 万），超出后清理最旧的请求记录及其原始正文，不影响按天聚合报表；同时受保留天数限制。'),
  },
  {
    key: 'dailyAggregateRetentionDays',
    id: 'settings-retention-daily',
    label: t('按天聚合保留天数'),
    unit: t('天'),
    kind: 'days',
    min: RETENTION_MIN,
    max: RETENTION_MAX,
    hint: t('报表页的热力图与按天趋势，默认 99999 天（近似全量保留），可填 1–99999 天。'),
  },
]

export function retentionShrinkHead(field: RetentionField, previous: number | null, value: number): string {
  if (field.kind === 'count') {
    return previous === null
      ? t('没能读到「{label}」的当前值，改为 {count} 条可能会删除超出的历史数据。', {
        label: field.label, count: value,
      })
      : t('「{label}」将从 {previous} 条改为 {count} 条。', { label: field.label, previous, count: value })
  }
  return previous === null
    ? t('没能读到「{label}」的当前值，改为 {days} 天可能会删除超出的历史数据。', {
      label: field.label, days: value,
    })
    : t('「{label}」将从 {previous} 天改为 {days} 天。', { label: field.label, previous, days: value })
}

export function retentionSavedMessage(field: RetentionField, value: number, shrinking: boolean): string {
  if (field.kind === 'count') {
    return shrinking
      ? t('✅ 数量上限已设为 {count} 条，超出部分已清理', { count: value })
      : t('✅ 数量上限已设为 {count} 条', { count: value })
  }
  return shrinking
    ? t('✅ 已保留 {days} 天，超出部分已清理', { days: value })
    : t('✅ 已保留 {days} 天', { days: value })
}
