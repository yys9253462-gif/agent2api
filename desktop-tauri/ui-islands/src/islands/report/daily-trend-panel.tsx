import * as React from 'react'
import { Card, CardContent, CardHeader, SegmentedControl, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@ui'
import { DailyTrendChart } from './report-recharts'
import { sourceName } from './source-stats'
import { SLICE_COLORS, type StatsDay, type StatsGroup, type StatsSummary } from '../report-charts'
import { t } from '../../i18n'

export type TrendDimension = 'models' | 'clients' | 'apiKeys'
export type TrendMetric = 'tokens' | 'requests'

const DIMENSIONS = [
  { value: 'models', label: t('模型') }, { value: 'clients', label: t('客户端') }, { value: 'apiKeys', label: t('API Key') },
]
const METRICS = [{ value: 'tokens', label: t('Token 用量') }, { value: 'requests', label: t('请求数') }]

export function ReportDailyTrend({ summary, loading }: { summary: StatsSummary | null; loading: React.ReactNode }) {
  const [dimension, setDimension] = React.useState<TrendDimension>('models')
  const [metric, setMetric] = React.useState<TrendMetric>('tokens')
  const [selected, setSelected] = React.useState('__all__')
  const options = [{ value: '__all__', label: t('全部') }, ...(summary?.[dimension] || []).map(row => ({
    value: dimension === 'apiKeys' ? row.filterValue || 'unknown' : row.id || row.model || '__unknown__',
    label: dimension === 'models' ? String(row.label || row.model || t('未知模型')) : sourceName(row, dimension),
  }))]
  if (dimension === 'apiKeys') {
    const counts = new Map<string, number>()
    for (const option of options) counts.set(option.label, (counts.get(option.label) || 0) + 1)
    for (const option of options) if ((counts.get(option.label) || 0) > 1) option.label += ` · ${option.value.replace(/^key:/, '')}`
  }
  if (!options.some(option => option.value === selected)) options.push({ value: selected, label: selected })
  const { series, colors } = React.useMemo(() => {
    const group: StatsGroup[] = summary?.[dimension] || []
    const names = new Map<string, string>()
    for (const row of group) names.set(String(dimension === 'apiKeys' ? row.filterValue : row.id || row.model || ''),
      dimension === 'models' ? String(row.label || row.model || t('未知模型')) : sourceName(row, dimension))
    if (dimension === 'apiKeys') {
      const counts = new Map<string, number>()
      for (const name of names.values()) counts.set(name, (counts.get(name) || 0) + 1)
      for (const [id, name] of names) if ((counts.get(name) || 0) > 1) names.set(id, `${name} · ${id.replace(/^key:/, '')}`)
    }
    const totals = new Map<string, number>()
    const series: StatsDay[] = (summary?.dailyTrend || []).map(day => {
      const items = (day[dimension] || []).filter(item => selected === '__all__' || String(item.id || item.model || '__unknown__') === selected)
      const slices = items.map(item => {
        const id = String(item.id ?? item.model ?? '')
        const label = names.get(id) || String(item.label || id || t('未记录'))
        const tokens = Number(metric === 'requests' ? item.requests : item.tokens) || 0
        totals.set(id, (totals.get(id) || 0) + tokens)
        return { ...item, id, label, tokens }
      })
      const requests = selected === '__all__' ? day.requests : items.reduce((sum, item) => sum + (Number(item.requests) || 0), 0)
      const tokens = selected === '__all__' ? metric === 'requests' ? day.requests : day.tokens : slices.reduce((sum, item) => sum + item.tokens, 0)
      return { ...day, requests, tokens, models: slices }
    })
    const colors = new Map<string, string>()
    const ordered = [...totals.entries()].sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    for (const [id] of ordered) {
      const name = names.get(id) || series.flatMap(day => day.models || []).find(item => item.id === id)?.label || id
      if (!colors.has(name)) colors.set(name, SLICE_COLORS[colors.size % SLICE_COLORS.length])
    }
    return { series, colors }
  }, [summary, dimension, metric, selected])
  const unknown = dimension === 'models' ? t('未知模型') : dimension === 'clients' ? t('未记录客户端') : t('历史未记录 Key')
  return (
    <Card render={<section />} className='panel'>
      <CardHeader className='panel-head'>
        <h2>{metric === 'tokens' ? t('按天 Token 趋势') : t('按天请求趋势')}</h2>
        <span className='panel-sub'>{summary ? `${summary.startDate || '—'} ～ ${summary.endDate || '—'}` : '—'}</span>
        <div className='head-actions'>
          <SegmentedControl options={DIMENSIONS} value={dimension} onValueChange={value => {
            setDimension(value as TrendDimension); setSelected('__all__')
          }}
            aria-label={t('趋势统计维度')} />
          <SegmentedControl options={METRICS} value={metric} onValueChange={value => setMetric(value as TrendMetric)}
            aria-label={t('趋势统计指标')} />
          <Select value={selected} onValueChange={value => setSelected(String(value ?? '__all__'))}>
            <SelectTrigger className='min-w-[120px] max-w-[240px]' aria-label={t('趋势统计维度')}>
              <SelectValue>{options.find(option => option.value === selected)?.label || t('全部')}</SelectValue>
            </SelectTrigger>
            <SelectContent>{options.map(option => <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>)}</SelectContent>
          </Select>
        </div>
      </CardHeader>
      <CardContent className='panel-body'>
        <div className='chart-wrap tall' id='report-daily-trend'>
          {summary ? <DailyTrendChart series={series} colors={colors} metric={metric} unknownLabel={unknown} /> : loading}
        </div>
      </CardContent>
    </Card>
  )
}
