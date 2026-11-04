import * as React from 'react'
import {
  Badge, Button, Card, CardContent, CardFooter, CardHeader, Collapsible, CollapsibleContent,
  CollapsibleTrigger, Empty, Switch, confirmDialog,
} from '@ui'
import { t } from '../i18n'
import { patch, removeScheduleEntry, runScheduleEntry, saveScheduleEntry, store } from './checkin-store'
import type { CheckinCenter, ScheduleEntry } from './checkin-types'
import { ProviderLogo } from './checkin-row'
import { CheckinScheduleEditor } from './checkin-schedule-editor'
import { CheckinScheduleExtras } from './checkin-schedule-extras'
import { CheckinScheduleHeading, CheckinScheduleMaster, CheckinScheduleRules } from './checkin-schedule-common'
import {
  beijingScheduleTime, checkinEnd, checkinRangeText, goCheckinTab, groupScheduleRows, scheduleGroupLabel,
  scheduleName, scheduleOptions, schedulePayload, scheduleRows, type ScheduleOption, type ScheduleTimeGroup,
} from './checkin-schedule-model'

function ScheduleGroup({ group, entry, masterOn, options }: {
  group: ScheduleTimeGroup
  entry: ScheduleEntry
  masterOn: boolean
  options: ScheduleOption[]
}): React.ReactElement {
  const [open, setOpen] = React.useState(false)
  const fired = group.rows.filter(row => row.firedToday).length
  const overdue = group.rows.some(row => !row.firedToday && row.overdue)
  const paused = !masterOn || !entry.enabled
  const completed = fired === group.rows.length
  const accounts = group.rows.reduce((sum, row) => sum + (row.accountCount || 0), 0)
  const attempted = group.rows.reduce((sum, row) => sum + (row.attemptedCount || 0), 0)
  const partial = attempted > 0 && !completed
  const nextAccountAt = group.rows.filter(row => !row.firedToday).map(row => Number(row.nextAccountAt))
    .filter(at => Number.isFinite(at) && at > 0).sort((a, b) => a - b)[0]
  const status = accounts === 0 ? t('没有可执行账号') : completed ? t('主签到已执行') : paused ? t('已暂停')
    : partial ? t('区间内陆续签到') : overdue ? t('到点待执行') : t('等待签到区间')
  return <Collapsible open={open} onOpenChange={setOpen} className='ck-schedule-group'>
    <div className='ck-schedule-group-head'>
      <strong className='font-mono tabular-nums'>{checkinRangeText(group.time, group.timeEnd)}</strong>
      <Badge variant={completed && accounts > 0 ? 'success' : overdue && !paused ? 'brand' : 'outline'} shape='tag'>{status}</Badge>
    </div>
    <div className='ck-schedule-group-meta'><span>{scheduleGroupLabel(group, entry)}</span>
      <span>{t('{n} 个平台', { n: group.rows.length })}</span>
    </div>
    <div className='ck-schedule-group-logos'>
      {group.rows.slice(0, 6).map(row => <ProviderLogo key={row.id} id={row.id} label={t(row.label)} />)}
      {group.rows.length > 6 ? <span>+{group.rows.length - 6}</span> : null}
    </div>
    {nextAccountAt && !paused ? <p className='ck-schedule-hint'>{t('下一账号：{time}', { time: beijingScheduleTime(nextAccountAt) })}</p> : null}
    <div className='ck-schedule-group-foot'>
      <span>{accounts > 0 ? t('已执行 {n} / {total} 个账号', { n: attempted, total: accounts })
        : t('已执行 {n} / {total} 个平台', { n: fired, total: group.rows.length })}</span>
      <CollapsibleTrigger render={<Button variant='link' size='xs' />}>
        {open ? t('收起平台') : t('查看平台')}
      </CollapsibleTrigger>
    </div>
    <CollapsibleContent className='ck-schedule-group-details'>
      {group.rows.map(row => {
        const available = options.find(option => option.id === row.id)?.available
        return <div className='ck-schedule-group-detail' key={row.id}>
          <span>{t(row.label)}</span><small>{!available ? t('插件未就绪，范围仍保留') : row.accountCount === 0
            ? t('没有可执行账号') : row.firedToday ? t('今日已执行') : paused ? t('已暂停') : row.overdue ? t('待执行') : t('等待签到区间')}
            {Number(row.accountCount) > 0 ? <span className='ck-schedule-detail-progress'>
              {t('已执行 {n} / {total} 个账号', { n: row.attemptedCount || 0, total: row.accountCount || 0 })}
            </span> : null}
          </small>
        </div>
      })}
    </CollapsibleContent>
  </Collapsible>
}

function SchedulePlan({ entry, masterOn, options }: {
  entry: ScheduleEntry
  masterOn: boolean
  options: ScheduleOption[]
}): React.ReactElement {
  const rows = scheduleRows(entry, options)
  const groups = groupScheduleRows(rows)
  const fired = rows.filter(row => row.firedToday).length
  const accounts = rows.reduce((sum, row) => sum + (row.accountCount || 0), 0)
  const attempted = rows.reduce((sum, row) => sum + (row.attemptedCount || 0), 0)
  const name = scheduleName(entry)
  const result = entry.lastResult
  const running = store.runningScheduleId === entry.id
  const remove = async () => {
    if (!await confirmDialog(t('确定删除「{name}」？对应的定时补签与活动重试也会一并移除，签到记录保留。', { name }))) return
    void removeScheduleEntry(entry.id)
  }
  const run = async () => {
    if (!await confirmDialog(t('立即执行「{name}」会手动签到整个计划范围，不等待平台时点。窗口未开的平台可能仍无法领取。', { name }), { destructive: false })) return
    void runScheduleEntry(entry.id)
  }
  return <Card render={<section />} className='ck-schedule-plan'>
    <CardHeader className='ck-schedule-plan-head'>
      <div className='ck-schedule-plan-name'><h3>{name}</h3>
        <Badge variant={entry.enabled ? 'brand' : 'outline'} shape='tag'>{entry.enabled ? t('每天执行') : t('已停用')}</Badge>
        <p>{t('默认 {time} · {n} 个平台 · {groups} 组签到时间', { time: checkinRangeText(entry.time, checkinEnd(entry.time, entry.timeEnd)), n: rows.length, groups: groups.length })}</p>
      </div>
      <div className='ck-schedule-plan-actions'>
        <Switch checked={entry.enabled} disabled={store.saving} aria-label={t('启用签到计划 {name}', { name })}
          onCheckedChange={enabled => void saveScheduleEntry(schedulePayload(entry, { enabled: enabled === true }), { closeEditor: false, silent: true })} />
        <Button size='sm' variant='outline' disabled={store.runningScheduleId !== null || store.center?.auto.running} onClick={() => void run()}>
          {running ? t('执行中…') : t('立即执行')}
        </Button>
        <Button size='sm' variant='ghost' disabled={store.saving} onClick={() => patch({ editingId: entry.id })}>{t('编辑')}</Button>
        <Button size='sm' variant='ghost' className='text-destructive' disabled={store.saving} onClick={() => void remove()}>{t('删除')}</Button>
      </div>
    </CardHeader>
    <CardContent className='ck-schedule-plan-content grid gap-3'>
      <div className='ck-schedule-today'><strong>{t('今日主签到')}</strong>
        <span>{accounts > 0 ? t('已执行 {n} / {total} 个账号', { n: attempted, total: accounts })
          : t('已执行 {n} / {total} 个平台', { n: fired, total: rows.length })}</span>
        {rows.some(row => !row.firedToday && row.overdue) && masterOn && entry.enabled
          ? <Badge variant='warning' shape='tag'>{t('有平台到点待执行')}</Badge> : null}
      </div>
      <div className='ck-schedule-time-groups'>
        {groups.map(group => <ScheduleGroup key={group.time} group={group} entry={entry} masterOn={masterOn} options={options} />)}
      </div>
      <div className='ck-schedule-plan-note'><span>{t('这里只管理签到时间；补签与活动窗口重试在独立页设置。')}</span>
        <Button size='xs' variant='link' onClick={() => goCheckinTab('makeup')}>{t('设置定时补签')}</Button>
      </div>
    </CardContent>
    <CardFooter className='ck-schedule-plan-footer'>
      <span>{result?.at ? t('最近执行：{time}', { time: beijingScheduleTime(result.at) }) : t('尚未执行')}</span>
      {result ? <span>{t('最近结果：成功 {n} · 保活 {active} · 失败 {failed}', {
        n: Number(result.succeeded) || 0, active: Number(result.active) || 0, failed: Number(result.failedCount) || 0,
      })}</span> : null}
      <Button size='xs' variant='link' onClick={() => goCheckinTab('records')}>{t('查看签到记录')}</Button>
    </CardFooter>
  </Card>
}

export function ScheduleTab({ center }: { center: CheckinCenter }): React.ReactElement {
  const auto = center.auto
  const options = scheduleOptions(auto)
  const entries = auto.schedules ?? []
  return <div className='ck-schedule-main'>
    <CheckinScheduleHeading title={t('定时签到')} description={t('设置每天的签到区间，各账号在区间内随机错峰执行。')}
      action={<Button size='sm' disabled={store.saving || entries.length >= 10 || !options.some(option => option.available)}
        onClick={() => patch({ editingId: 'new' })}>{t('新建签到计划')}</Button>} />
    <CheckinScheduleMaster auto={auto} />
    <div className='ck-schedule-section-heading'><h3>{t('签到计划')}</h3><span>{t('{n} 个计划 · 北京时间 UTC+8', { n: entries.length })}</span></div>
    {entries.map(entry => <SchedulePlan key={entry.id} entry={entry} options={options} masterOn={auto.enabled} />)}
    {!entries.length ? <Empty className='ck-schedule-empty'>
      <p>{t('还没有签到计划，先选择平台和默认时间。')}</p>
      {!options.some(option => option.available) ? <Button variant='outline' size='sm' onClick={() => window.wbApp?.showPage?.('plugins')}>{t('去插件页安装签到能力')}</Button> : null}
    </Empty> : null}
    <CheckinScheduleRules />
    <CheckinScheduleExtras center={center} />
    <CheckinScheduleEditor entries={entries} options={options} defaultRetryTime={auto.defaultRetryTime || '00:10'} />
  </div>
}
