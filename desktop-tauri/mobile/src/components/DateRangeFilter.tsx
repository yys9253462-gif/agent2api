import { Input, Toggle } from '@ui'
import { DATE_RANGES, dayKey, updateCustomRange, type CustomDates, type DateRange } from '../../../ui-shared/date-range'
import { t } from '../i18n'

const LABELS: Record<DateRange, string> = {
  today: t('今天'), yesterday: t('昨天'), 7: t('7 天'), 30: t('30 天'),
  month: t('本月'), all: t('全部'), custom: t('自定义'),
}

export function DateRangeFilter({ range, dates, onRangeChange, onDatesChange, label }: {
  range: DateRange; dates: CustomDates; onRangeChange: (range: DateRange) => void
  onDatesChange: (dates: CustomDates) => void; label: string
}) {
  const today = dayKey(new Date())
  const change = (patch: Partial<CustomDates>) => {
    const next = updateCustomRange(dates, patch)
    if (next !== dates) onDatesChange(next)
  }
  return <>
    <div className='m-chips py-2' role='group' aria-label={label}>
      {DATE_RANGES.map(value => <Toggle key={value} variant='outline' size='default'
        className='m-chip rounded-pill shrink-0' pressed={range === value} onClick={() => onRangeChange(value)}>
        {LABELS[value]}
      </Toggle>)}
    </div>
    {range === 'custom' ? <div className='m-section flex items-center gap-2 !mt-0 pb-2'>
      <Input type='date' value={dates.start} max={dates.end < today ? dates.end : today} className='min-w-0 flex-1'
        aria-label={t('开始日期')} title={t('起始日期（含当天）')}
        onChange={event => change({ start: event.currentTarget.value })} />
      <span className='text-muted-foreground' aria-hidden='true'>～</span>
      <Input type='date' value={dates.end} min={dates.start} max={today} className='min-w-0 flex-1'
        aria-label={t('结束日期')} title={t('结束日期（含当天）')}
        onChange={event => change({ end: event.currentTarget.value })} />
    </div> : null}
  </>
}
