import { usePlotArea, useXAxisScale, useYAxisScale, ZIndexLayer, type PieLabelRenderProps } from 'recharts'
import { formatPercent, formatTokens, textWidth } from '../report-charts'
import type { CacheTrendRow, DailyTrendRow } from './chart-data'

type LabelBox = { left: number; right: number; top: number; bottom: number }
type PointLabel = {
  key: string; x: number; y: number; text: string; cls: string
  anchor: 'start' | 'middle' | 'end'
}

function collides(box: LabelBox, placed: LabelBox[]): boolean {
  return placed.some(item => box.left < item.right + 3 && box.right > item.left - 3
    && box.top < item.bottom + 2 && box.bottom > item.top - 2)
}

function Labels({ items }: { items: PointLabel[] }) {
  return (
    <ZIndexLayer zIndex={2000}>
      <g pointerEvents='none'>
        {items.map(item => (
          <text x={item.x} y={item.y} textAnchor={item.anchor}
            className={`chart-label ${item.cls}`} key={item.key}>{item.text}</text>
        ))}
      </g>
    </ZIndexLayer>
  )
}

export function CachePointLabels({ rows }: { rows: CacheTrendRow[] }) {
  const plot = usePlotArea()
  const xScale = useXAxisScale()
  const rateScale = useYAxisScale()
  const tokenScale = useYAxisScale('tokens')
  if (!plot || !xScale || !rateScale) return null
  const items: PointLabel[] = []
  const boxes: LabelBox[] = []
  const add = (row: CacheTrendRow, index: number, cls: 'tokens' | 'rate') => {
    if (cls === 'rate' ? !row.used : row.totalTokens <= 0) return
    const x = xScale(row.hour)
    const pointY = cls === 'rate' ? rateScale(row.rate) : tokenScale?.(row.totalTokens)
    if (x == null || pointY == null) return
    const text = cls === 'rate' ? formatPercent(row.rate) : formatTokens(row.totalTokens)
    const width = textWidth(text)
    const anchor = x - width / 2 < plot.x ? 'start' : x + width / 2 > plot.x + plot.width ? 'end' : 'middle'
    const left = anchor === 'start' ? x : anchor === 'end' ? x - width : x - width / 2
    let y = cls === 'rate' ? pointY + 17 : pointY - 10
    if (y > plot.y + plot.height) y = pointY - 8
    if (y - 11 < plot.y - 18) y = pointY + 17
    const box = { left, right: left + width, top: y - 11, bottom: y + 2 }
    if (collides(box, boxes)) return
    boxes.push(box)
    items.push({ key: `${cls}-${index}`, x, y, text, cls, anchor })
  }
  rows.forEach((row, index) => add(row, index, 'tokens'))
  rows.forEach((row, index) => add(row, index, 'rate'))
  return <Labels items={items} />
}

export function DailyPointLabels({ rows, formatValue = formatTokens }: {
  rows: DailyTrendRow[]; formatValue?: (value: number) => string
}) {
  const plot = usePlotArea()
  const xScale = useXAxisScale()
  const yScale = useYAxisScale()
  if (!plot || !xScale || !yScale || !rows.length) return null
  const ranked = rows.map((row, index) => ({ row, index })).filter(({ row }) => row.total > 0)
  const step = plot.width / rows.length
  const allFit = ranked.every(({ row }) => textWidth(formatValue(row.total)) + 6 <= step)
  if (!allFit) ranked.sort((left, right) => right.row.total - left.row.total)
  const items: PointLabel[] = []
  const boxes: LabelBox[] = []
  for (const { row, index } of ranked) {
    if (!allFit && items.length >= 8) break
    const x = xScale(row.date)
    const pointY = yScale(row.total)
    if (x == null || pointY == null) continue
    const text = formatValue(row.total)
    const width = textWidth(text)
    const anchor = x - width / 2 < plot.x ? 'start' : x + width / 2 > plot.x + plot.width ? 'end' : 'middle'
    const left = anchor === 'start' ? x : anchor === 'end' ? x - width : x - width / 2
    const y = pointY - 8
    const box = { left, right: left + width, top: y - 11, bottom: y + 2 }
    if (collides(box, boxes)) continue
    boxes.push(box)
    items.push({ key: `day-${index}`, x, y, text, cls: 'total', anchor })
  }
  return <Labels items={items} />
}

export function renderDonutLabel({ cx, cy, midAngle, innerRadius, outerRadius, percent }: PieLabelRenderProps) {
  if (percent == null || percent < 0.03 || midAngle == null) return null
  const radius = innerRadius + (outerRadius - innerRadius) * 0.6
  const angle = -midAngle * Math.PI / 180
  return (
    <text x={cx + radius * Math.cos(angle)} y={cy + radius * Math.sin(angle)}
      className='donut-label' textAnchor='middle' dominantBaseline='central'>
      {`${Math.round(percent * 100)}%`}
    </text>
  )
}
