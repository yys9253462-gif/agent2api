export type LimiterJoin = 'and' | 'or'
export type LimiterMetric = 'remaining' | 'remaining_percent' | 'used' | 'used_percent' | 'tokens' | 'requests'
export type LimiterComparison = 'lt' | 'lte' | 'gte'
export type LimiterWindow = {
  reset: 'fixed' | 'rolling' | 'daily' | 'weekly' | 'monthly' | 'total'
  seconds?: number
  startsAt?: number
  endsAt?: number
  utcOffsetMinutes?: number | null
}
export type QuotaSource = {
  id: string
  label: string
  provider?: string
  channel?: string
  planId?: string
  planName?: string
  models?: string[]
  unit?: string
  remaining?: number | null
  total?: number | null
  used?: number | null
  remainingPercent?: number | null
  startsAt?: number
  endsAt?: number
  resetsAt?: number
  window?: LimiterWindow | null
  unlimited?: boolean
  localEstimate?: boolean
  localOnly?: boolean
}
export type LimiterCondition = {
  join: LimiterJoin
  sourceId: string
  source?: QuotaSource | null
  models: string[]
  metric: LimiterMetric
  comparison: LimiterComparison
  threshold: number
  window?: LimiterWindow | null
}
export type LimiterGroup = {
  enabled: boolean
  action: 'skip' | 'disable'
  target: 'models' | 'account'
  conditions: LimiterCondition[]
}
export type LimiterReading = {
  known: boolean
  value: number | null
  hit: boolean
  local: boolean
  models: string[]
  modelValues: Record<string, number>
  windowStart: number
  resetsAt: number
  reason: string
}
export type LimiterGroupState = {
  hit: boolean
  account: boolean
  models: string[]
  conditions: LimiterReading[]
}
export type LimiterSnapshot = {
  tokenAt?: unknown
  limiterStates?: Record<string, LimiterGroupState[]>
  quotaSources?: Record<string, QuotaSource[]>
  limiterModels?: Record<string, Array<{ id: string; name?: string }>>
}
