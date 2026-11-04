import * as React from 'react'
import { Tooltip, TooltipContent, TooltipProvider } from '@ui'
import type { TooltipContentProps } from 'recharts'
import { dayLabel, formatInt, formatPercent, formatTokens, hourText, type DonutDatum } from '../report-charts'
import { t } from '../../i18n'
import type { CacheTrendRow, DailyTrendRow, ModelSlice } from './chart-data'

type ChartTooltipProps = Pick<TooltipContentProps, 'active' | 'payload' | 'coordinate'> & {
  container: React.RefObject<HTMLDivElement | null>
}

function ChartTooltip({ active, coordinate, container, children }: Omit<ChartTooltipProps, 'payload'> & {
  children: React.ReactNode
}) {
  const [dismissed, setDismissed] = React.useState(false)
  React.useEffect(() => { setDismissed(false) }, [active, coordinate?.x, coordinate?.y])
  const anchor = React.useMemo(() => ({
    contextElement: container.current || undefined,
    getBoundingClientRect: () => {
      const box = container.current?.getBoundingClientRect()
      if (!box || !box.width || !box.height) return new DOMRect(-1000, -1000, 1, 1)
      return new DOMRect(box.left + (coordinate?.x || 0), box.top + (coordinate?.y || 0), 1, 1)
    },
  }), [container, coordinate?.x, coordinate?.y])
  return (
    <TooltipProvider>
      <Tooltip open={active && !dismissed} disableHoverablePopup onOpenChange={open => setDismissed(!open)}>
        {/* 卡片会裁剪子内容，读数沿用组件库的固定定位浮层。 */}
        <TooltipContent anchor={anchor} side='right' sideOffset={24} variant='plain'
          className='report-chart-tooltip pointer-events-none'>
          {children}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}

function TokenValue({ value }: { value: number }) {
  const full = formatInt(value)
  const compact = formatTokens(value)
  return (
    <b className='report-chart-tooltip-value'>
      {full}
      {compact !== full ? <><span className='report-chart-tooltip-sep'>|</span>{compact}</> : null}
    </b>
  )
}

function TooltipRow({ name, value, fill }: { name: string; value: React.ReactNode; fill?: string }) {
  return (
    <div className='report-chart-tooltip-row'>
      {fill ? <span className='report-chart-tooltip-dot' style={{ background: fill }} /> : null}
      <span className='report-chart-tooltip-name'>{name}</span>
      {value}
    </div>
  )
}

function ModelRows({ rows, requests = false }: { rows: ModelSlice[]; requests?: boolean }) {
  return (
    <>
      {rows.length ? <div className='report-chart-tooltip-divider' /> : null}
      {rows.slice(0, 8).map(model => (
        <TooltipRow name={model.name} fill={model.fill} value={requests
          ? <b className='report-chart-tooltip-value'>{formatInt(model.tokens)}</b>
          : <TokenValue value={model.tokens} />} key={model.name} />
      ))}
      {rows.length > 8 ? <div className='report-chart-tooltip-sub'>{t('…还有 {n} 项', { n: rows.length - 8 })}</div> : null}
    </>
  )
}

export function DailyTooltip(props: ChartTooltipProps & { metric?: 'tokens' | 'requests' }) {
  const row = props.payload.find(item => item.payload)?.payload as DailyTrendRow | undefined
  if (!props.active || !row) return null
  return (
    <ChartTooltip {...props}>
      <div className='report-chart-tooltip-title'>{dayLabel(row.date)}</div>
      {props.metric !== 'requests' ? <TooltipRow name={t('总 Token')} value={<TokenValue value={row.total} />} /> : null}
      <TooltipRow name={t('总请求数')} value={<b className='report-chart-tooltip-value'>{formatInt(row.requests)}</b>} />
      <ModelRows rows={row.models} requests={props.metric === 'requests'} />
    </ChartTooltip>
  )
}

export function CacheTooltip(props: ChartTooltipProps) {
  const row = props.payload.find(item => item.payload)?.payload as CacheTrendRow | undefined
  if (!props.active || !row) return null
  return (
    <ChartTooltip {...props}>
      <div className='report-chart-tooltip-title'>{dayLabel(row.hour.slice(0, 10))} {hourText(row.hour)}</div>
      <TooltipRow name={t('缓存命中率')}
        value={<b className='report-chart-tooltip-value'>{formatPercent(row.rate)}</b>} />
      <div className='report-chart-tooltip-sub'>
        {t('命中 {hit} / 输入 {input}', { hit: formatInt(row.hitTokens), input: formatInt(row.inputTokens) })}
      </div>
      <TooltipRow name={t('总 Token')} value={<TokenValue value={row.totalTokens} />} />
      <ModelRows rows={row.models} />
    </ChartTooltip>
  )
}

export function DonutTooltip(props: ChartTooltipProps) {
  const row = props.payload.find(item => item.payload)?.payload as DonutDatum | undefined
  if (!props.active || !row) return null
  return (
    <ChartTooltip {...props}>
      <div className='report-chart-tooltip-title'>{row.name} · {formatPercent(row.percentage)}</div>
      <TooltipRow name={t('Token')} fill={row.fill} value={<TokenValue value={row.tokens} />} />
      <TooltipRow name={t('总请求数')} value={<b className='report-chart-tooltip-value'>{formatInt(row.requests)}</b>} />
    </ChartTooltip>
  )
}
