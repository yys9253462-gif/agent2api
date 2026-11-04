import {
  TIP_DOT_FALLBACK, axisMax, parseDay,
  type StatsDay, type StatsHour, type StatsSlice,
} from '../report-charts'
import { t } from '../../i18n'

export type ModelSlice = { name: string; tokens: number; fill: string }
export type DailyTrendRow = {
  date: string; total: number; requests: number
  models: ModelSlice[]; segments: ModelSlice[]
  [key: `slot${number}`]: number
}
export type CacheTrendRow = {
  hour: string; rate: number; totalTokens: number; hitTokens: number; inputTokens: number
  used: boolean; models: ModelSlice[]
}
export type ModelLegendRow = { name: string; fill: string }

function amount(value: unknown): number {
  const number = Number(value)
  return Number.isFinite(number) && number > 0 ? number : 0
}

export function modelSlices(list: StatsSlice[] | undefined, colors: Map<string, string>): ModelSlice[] {
  const rows = new Map<string, ModelSlice>()
  for (const item of Array.isArray(list) ? list : []) {
    const name = String(item?.label ?? item?.model ?? '').trim() || t('未知模型')
    const tokens = amount(item?.tokens)
    if (!tokens) continue
    const previous = rows.get(name)
    if (previous) previous.tokens += tokens
    else rows.set(name, { name, tokens, fill: colors.get(name) || TIP_DOT_FALLBACK })
  }
  return [...rows.values()].sort((left, right) => right.tokens - left.tokens || left.name.localeCompare(right.name))
}

export function dailyTrendData(series: StatsDay[], colors: Map<string, string>, unknownLabel = t('未知模型')): DailyTrendRow[] {
  return series.map(day => {
    const total = amount(day.tokens)
    const models = modelSlices(day.models, colors)
    const sum = models.reduce((value, model) => value + model.tokens, 0)
    const segments = models.map(model => ({ ...model }))
    // 老聚合可能只有总量；缺失的模型明细不能让堆叠柱少画一部分用量。
    if (sum < total) {
      const name = unknownLabel;
      const unknown = segments.find(model => model.name === name)
      if (unknown) unknown.tokens += total - sum
      else segments.push({ name, tokens: total - sum, fill: colors.get(name) || TIP_DOT_FALLBACK })
    } else if (sum > total) {
      // 明细与合计不一致时仍以合计画柱，提示框保留原始模型读数。
      segments.splice(0, segments.length, { name: t('总 Token'), tokens: total, fill: TIP_DOT_FALLBACK })
    }
    segments.sort((left, right) => right.tokens - left.tokens || left.name.localeCompare(right.name))
    const row: DailyTrendRow = {
      date: String(day.date || ''), total, requests: amount(day.requests), models,
      segments: segments.filter(model => model.tokens > 0),
    }
    row.segments.forEach((model, index) => { row[`slot${index}`] = model.tokens })
    return row
  })
}

export function dailyModelLegend(rows: DailyTrendRow[], colors: Map<string, string>): ModelLegendRow[] {
  const used = new Map<string, string>()
  for (const row of rows) for (const model of row.segments) used.set(model.name, model.fill)
  const legend: ModelLegendRow[] = []
  for (const [name, fill] of colors) {
    if (!used.has(name)) continue
    legend.push({ name, fill })
    used.delete(name)
  }
  for (const [name, fill] of used) legend.push({ name, fill })
  return legend
}

export function cacheTrendData(series: StatsHour[], colors: Map<string, string>): CacheTrendRow[] {
  return series.map(hour => {
    const hitTokens = amount(hour.hitTokens)
    const inputTokens = amount(hour.inputTokens)
    return {
      hour: String(hour.hour || ''), rate: amount(hour.rate),
      totalTokens: amount(hour.totalTokens), hitTokens, inputTokens,
      used: inputTokens + hitTokens > 0, models: modelSlices(hour.models, colors),
    }
  })
}

export function chartAxis(peak: number, minimum = 0) {
  const max = Math.max(minimum, axisMax(peak > 0 ? peak : minimum || 1))
  return { max, ticks: Array.from({ length: 5 }, (_, index) => max * index / 4) }
}

export function dateTick(value: string): string {
  const day = parseDay(value)
  return `${day.getMonth() + 1}/${day.getDate()}`
}
