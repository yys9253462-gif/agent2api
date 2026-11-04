import * as React from 'react'
import {
  Button, Dialog, DialogBody, DialogContent, DialogFooter, DialogHeader, DialogTitle,
  Input, Label, Switch, Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@ui'
import { t } from '../i18n'
import { saveScheduleEntry, store } from './checkin-store'
import type { ScheduleEntry } from './checkin-types'
import {
  activityRetryEnabled, checkinRangeText, groupScheduleRows, previewMakeupTime, scheduleName, schedulePayload,
  scheduleRows, validScheduleTime, type ScheduleOption,
} from './checkin-schedule-model'

export type MakeupEditorKind = 'makeup' | 'retry'

export function CheckinMakeupEditor({ entry, kind, options, defaultRetryTime, onClose }: {
  entry: ScheduleEntry
  kind: MakeupEditorKind
  options: ScheduleOption[]
  defaultRetryTime: string
  onClose(): void
}): React.ReactElement {
  const isMakeup = kind === 'makeup'
  const [enabled, setEnabled] = React.useState(isMakeup ? !!entry.retryTime : activityRetryEnabled(entry))
  const [time, setTime] = React.useState(entry.retryTime || defaultRetryTime)
  const [gap, setGap] = React.useState(String(Math.max(0, entry.retryGapMinutes)))
  const [limit, setLimit] = React.useState(String(entry.retryLimit || 3))
  const gapValue = Number(gap)
  const limitValue = Number(limit)
  const gapValid = gap.trim() !== '' && Number.isInteger(gapValue) && gapValue >= 0 && gapValue <= 120
  const limitValid = limit.trim() !== '' && Number.isInteger(limitValue) && limitValue >= 1 && limitValue <= 20
  const canSave = !store.saving && (isMakeup
    ? !enabled || validScheduleTime(time) && gapValid
    : !enabled || gapValid && gapValue > 0 && limitValid)
  const rows = scheduleRows(entry, options)
  const groups = groupScheduleRows(rows)
  const submit = async () => {
    if (!canSave) return
    const changes = isMakeup
      ? { retryTime: enabled ? time : '', ...(enabled ? { retryGapMinutes: gapValue } : {}) }
      : { retryLimit: enabled ? limitValue : 0, ...(enabled ? { retryGapMinutes: gapValue } : {}) }
    const saved = await saveScheduleEntry(schedulePayload(entry, changes), { closeEditor: false })
    if (saved) onClose()
  }

  return <Dialog open onOpenChange={open => { if (!open && !store.saving) onClose() }}>
    <DialogContent className='ck-makeup-editor-dialog w-[min(700px,calc(100vw-48px))] max-[767px]:w-[calc(100vw-24px)]'>
      <DialogHeader><DialogTitle>{isMakeup ? t('设置定时补签') : t('设置活动窗口重试')}</DialogTitle></DialogHeader>
      <DialogBody>
        <p className='ck-schedule-hint'>{t('对应签到计划：{name}', { name: scheduleName(entry) })}</p>
        <div className='ck-makeup-setting'>
          <div><h3>{isMakeup ? t('开启定时补签') : t('开启活动窗口重试')}</h3>
            <p>{isMakeup ? t('每天补查一轮，只处理今天未签到的账号。') : t('仅处理窗口未开或活动暂未下发的账号，不对所有错误反复执行。')}</p>
          </div>
          <Switch checked={enabled} disabled={store.saving} onCheckedChange={next => {
            setEnabled(next === true)
            if (!isMakeup && next && gapValue <= 0) setGap('10')
          }} aria-label={isMakeup ? t('开启定时补签') : t('开启活动窗口重试')} />
        </div>
        <div className='ck-makeup-editor-fields'>
          <Label className='ck-schedule-field grid items-stretch'><span>{isMakeup ? t('区间结束后等待（分钟）') : t('重试间隔（分钟）')}</span>
            <Input type='number' value={gap} min={isMakeup ? 0 : 1} max={120} step={1}
              disabled={!enabled || store.saving} onChange={event => setGap(event.currentTarget.value)} />
          </Label>
          {isMakeup ? <Label className='ck-schedule-field grid items-stretch'><span>{t('补签不早于')}</span>
            <Input type='time' value={time} className='font-mono tabular-nums' disabled={!enabled || store.saving}
              onChange={event => setTime(event.currentTarget.value)} />
          </Label> : <Label className='ck-schedule-field grid items-stretch'><span>{t('每账号每天最多重试（次）')}</span>
            <Input type='number' value={limit} min={1} max={20} step={1} disabled={!enabled || store.saving}
              onChange={event => setLimit(event.currentTarget.value)} />
          </Label>}
        </div>
        {enabled && !canSave && !store.saving ? <p className='ck-schedule-error'>{t('请检查时间、间隔和次数是否在允许范围内。')}</p> : null}
        {isMakeup ? <>
          <p className='ck-schedule-hint'>{t('补签取「主签到区间结束加等待间隔」与「补签不早于」中较晚的时刻，不会提前补签尚未轮到的账号。')}</p>
          <Table className='ck-makeup-preview-table'>
            <TableHeader><TableRow><TableHead>{t('平台批次')}</TableHead><TableHead>{t('签到区间')}</TableHead><TableHead>{t('实际补签')}</TableHead></TableRow></TableHeader>
            <TableBody>{groups.map(group => <TableRow key={group.time}>
              <TableCell>{group.rows.map(row => t(row.label)).join('、')}</TableCell>
              <TableCell className='font-mono tabular-nums'>{checkinRangeText(group.time, group.timeEnd)}</TableCell>
              <TableCell className='font-mono tabular-nums'>{enabled && gapValid
                ? previewMakeupTime(group.timeEnd || group.time, time, gapValue) || '—' : t('已关闭')}</TableCell>
            </TableRow>)}</TableBody>
          </Table>
        </> : null}
        <div className='ck-schedule-editor-note'>{t('当前网关中，补签等待间隔与活动重试间隔共用；修改间隔会同步影响两者，开关与重试次数仍分别设置。')}</div>
        {isMakeup ? <p className='ck-schedule-hint'>{t('间隔填 0 会关闭活动窗口重试；定时补签仍至少等待 1 分钟，不会一起关闭。')}</p>
          : <>
            <p className='ck-schedule-hint'>{t('次数按账号每天单独计算：重试到上限的账号今天不再重试，明天自动重置。')}</p>
            <p className='ck-schedule-hint'>{t('关闭活动窗口重试不会关闭定时补签。插件提供下次重试时刻时，以插件返回的时间为准。')}</p>
          </>}
      </DialogBody>
      <DialogFooter><Button size='sm' variant='outline' disabled={store.saving} onClick={onClose}>{t('取消')}</Button>
        <Button size='sm' disabled={!canSave} onClick={() => void submit()}>{store.saving ? t('保存中…') : t('保存设置')}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
}
