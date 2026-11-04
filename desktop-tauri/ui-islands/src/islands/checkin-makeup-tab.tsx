import * as React from 'react'
import { Badge, Button, Card, CardContent, CardFooter, CardHeader, Collapsible, CollapsibleContent, CollapsibleTrigger, Empty, Switch } from '@ui'
import { t } from '../i18n'
import { saveScheduleEntry, store } from './checkin-store'
import type { CheckinCenter, ScheduleEntry } from './checkin-types'
import {
  activityRetryEnabled, checkinRangeText, goCheckinTab, groupScheduleRows, makeupScheduleRows, scheduleName,
  scheduleOptions, schedulePayload, scheduleRows, type ScheduleOption, type ScheduleTimeGroup,
} from './checkin-schedule-model'
import { CheckinMakeupEditor, type MakeupEditorKind } from './checkin-makeup-editor'
import { CheckinScheduleHeading, CheckinScheduleMaster } from './checkin-schedule-common'

function MakeupGroup({ group, entry, options, paused }: {
  group: ScheduleTimeGroup
  entry: ScheduleEntry
  options: ScheduleOption[]
  paused: boolean
}): React.ReactElement {
  const [open, setOpen] = React.useState(false)
  const rows = scheduleRows(entry, options)
  const mainTimes = [...new Set(group.rows.map(row => rows.find(item => item.id === row.id))
    .filter(Boolean).map(row => checkinRangeText(row!.time, row!.timeEnd)))]
  return <Collapsible open={open} onOpenChange={setOpen} className='ck-schedule-group ck-makeup-group'>
    <div className='ck-schedule-group-head'><strong className='font-mono tabular-nums'>{group.time}</strong>
      <Badge variant='outline' shape='tag'>{paused ? t('已暂停') : t('补签检查')}</Badge>
    </div>
    <div className='ck-schedule-group-meta'><span>{t('{n} 个平台', { n: group.rows.length })}</span>
      <span>{t('每天补查一次')}</span>
    </div>
    <div className='ck-makeup-group-providers'>{group.rows.slice(0, 3).map(row => t(row.label)).join('、')}
      {group.rows.length > 3 ? ` · +${group.rows.length - 3}` : ''}
    </div>
    <div className='ck-schedule-group-foot'><span>{t('对应主签到：{times}', { times: mainTimes.join(' / ') })}</span>
      <CollapsibleTrigger render={<Button variant='link' size='xs' />}>{open ? t('收起平台') : t('查看平台')}</CollapsibleTrigger>
    </div>
    <CollapsibleContent className='ck-schedule-group-details'>
      {group.rows.map(row => <div className='ck-schedule-group-detail' key={row.id}>
        <span>{t(row.label)}</span><small className='font-mono tabular-nums'>{row.time}</small>
      </div>)}
    </CollapsibleContent>
  </Collapsible>
}

function MakeupPlan({ entry, options, masterOn, defaultRetryTime, onEdit }: {
  entry: ScheduleEntry
  options: ScheduleOption[]
  masterOn: boolean
  defaultRetryTime: string
  onEdit(): void
}): React.ReactElement {
  const enabled = !!entry.retryTime
  const paused = !masterOn || !entry.enabled
  const groups = groupScheduleRows(makeupScheduleRows(entry, options))
  const name = scheduleName(entry)
  return <Card render={<section />} className='ck-schedule-plan'>
    <CardHeader className='ck-schedule-plan-head'>
      <div className='ck-schedule-plan-name'><h3>{name}</h3>
        <Badge variant={enabled ? 'success' : 'outline'} shape='tag'>{enabled ? t('补签已开启') : t('补签已关闭')}</Badge>
        {paused && enabled ? <Badge variant='outline' shape='tag'>{t('随签到计划暂停')}</Badge> : null}
        <p>{t('对应签到计划 · 沿用 {n} 个平台范围，只补未签到账号', { n: entry.providers.length })}</p>
      </div>
      <div className='ck-schedule-plan-actions'>
        <Switch checked={enabled} disabled={store.saving} aria-label={t('{name}定时补签开关', { name })}
          onCheckedChange={next => void saveScheduleEntry(schedulePayload(entry, { retryTime: next ? defaultRetryTime : '' }), { closeEditor: false, silent: true })} />
        <Button size='sm' variant='outline' disabled={store.saving} onClick={onEdit}>{t('设置补签')}</Button>
      </div>
    </CardHeader>
    <CardContent className='ck-schedule-plan-content grid gap-3'>
      <div className='ck-makeup-rule'>
        <span>{t('主签到区间结束后等待 {n} 分钟', { n: Math.max(1, entry.retryGapMinutes) })}</span>
        {enabled ? <span>{t('补签不早于 {time}', { time: entry.retryTime })}</span> : null}
        <Badge variant='outline' shape='tag'>{t('已签到自动跳过')}</Badge>
      </div>
      {enabled ? <div className='ck-schedule-time-groups'>
        {groups.map(group => <MakeupGroup key={group.time} group={group} entry={entry} options={options} paused={paused} />)}
      </div> : <p className='ck-schedule-hint'>{t('补签已关闭，不影响主签到；开启后会自动生成各平台的补签安排。')}</p>}
    </CardContent>
    <CardFooter className='ck-schedule-plan-footer'><span>{t('实际时间由网关计算，晚窗口的平台会顺延补签；只处理当天未签到账号。')}</span></CardFooter>
  </Card>
}

function ActivityRetryPlan({ entry, masterOn, onEdit }: {
  entry: ScheduleEntry
  masterOn: boolean
  onEdit(): void
}): React.ReactElement {
  const enabled = activityRetryEnabled(entry)
  const active = masterOn && entry.enabled && enabled
  // 重试关着时这两个数字都不必提：`retryAccounts` 里可能还留着旧名单，
  // 但它们不会被消费 —— 说出来只会让「重试关闭」的徽标自相矛盾。
  const count = enabled ? Number(entry.retryCount) || 0 : 0
  // 次数用完的账号已经不在 `retryCount` 里了（后端摘掉了），所以数字归零不等于
  // 「没事了」—— 这两种状态必须分开说，否则用户以为签到已经完成。
  const exhausted = enabled ? (entry.retryExhausted?.length ?? 0) : 0
  return <Card render={<section />} className='ck-activity-retry'>
    <CardHeader className='ck-schedule-plan-head'>
      <div className='ck-schedule-plan-name'><h3>{scheduleName(entry)}</h3>
        <Badge variant={active ? 'brand' : 'outline'} shape='tag'>{!enabled ? t('重试关闭') : active ? t('重试开启') : t('重试已暂停')}</Badge>
        <p>{enabled ? t('间隔 {gap} 分钟 · 每账号每天最多重试 {limit} 次', { gap: entry.retryGapMinutes, limit: entry.retryLimit })
          : t('不自动重试等待中的活动')}</p>
      </div>
      <Button size='sm' variant='ghost' disabled={store.saving} onClick={onEdit}>{t('调整重试')}</Button>
    </CardHeader>
    <CardContent className='grid gap-3'>
      {!enabled ? <p className='ck-schedule-hint'>{t('活动窗口重试已关闭，等待窗口或活动下发的账号不会自动再试。')}</p> : <>
        {count > 0 ? <div className='ck-activity-retry-notice' role='status'>
          <Badge variant='warning' shape='tag'>{t('{n} 个账号等待窗口或活动', { n: count })}</Badge>
          <span>{t('只重试这些账号，不会重签整个计划。具体原因见签到记录。')}</span>
          <Button size='xs' variant='link' onClick={() => goCheckinTab('records')}>{t('查看签到记录')}</Button>
        </div> : null}
        {exhausted > 0 ? <div className='ck-activity-retry-notice' role='status'>
          <Badge variant='outline' shape='tag'>{t('{n} 个账号今天的重试次数已用完', { n: exhausted })}</Badge>
          <span>{t('今天不再自动重试，明天自动重置。要现在再试一次，用签到计划里的「立即执行」。')}</span>
          <Button size='xs' variant='link' onClick={() => goCheckinTab('records')}>{t('查看签到记录')}</Button>
        </div> : null}
        {count === 0 && exhausted === 0
          ? <p className='ck-schedule-hint'>{t('当前没有等待窗口或活动下发的账号。')}</p> : null}
      </>}
    </CardContent>
  </Card>
}

export function MakeupTab({ center }: { center: CheckinCenter }): React.ReactElement {
  const [editing, setEditing] = React.useState<{ id: string; kind: MakeupEditorKind } | null>(null)
  const auto = center.auto
  const options = scheduleOptions(auto)
  const entries = auto.schedules ?? []
  const entry = editing ? entries.find(item => item.id === editing.id) : undefined
  const close = () => setEditing(null)
  return <div className='ck-schedule-main'>
    <CheckinScheduleHeading title={t('定时补签')} description={t('补签单独设置，只检查今天还没有签到的账号。')}
      action={<Button size='sm' variant='outline' onClick={() => goCheckinTab('schedule')}>{t('管理定时签到')}</Button>} />
    <CheckinScheduleMaster auto={auto} />
    <div className='ck-schedule-section-heading'><h3>{t('补签计划')}</h3>
      <span>{t('每个签到计划对应一组补签设置')}</span>
    </div>
    <div className='ck-schedule-editor-note'>{t('定时签到和定时补签分开管理。补签沿用对应签到计划的平台范围，不重复领取已完成的账号。')}</div>
    {entries.map(entry => <MakeupPlan key={entry.id} entry={entry} options={options} masterOn={auto.enabled}
      defaultRetryTime={auto.defaultRetryTime || '00:10'} onEdit={() => setEditing({ id: entry.id, kind: 'makeup' })} />)}
    {!entries.length ? <Empty className='ck-schedule-empty'>
      <p>{t('先创建签到计划，再为它设置定时补签。')}</p>
      <Button size='sm' variant='outline' onClick={() => goCheckinTab('schedule')}>{t('管理定时签到')}</Button>
    </Empty> : null}
    {entries.length ? <>
      <CheckinScheduleHeading title={t('活动窗口重试')} description={t('与定时补签不同，仅处理窗口未开或活动暂未下发。')} />
      {entries.map(entry => <ActivityRetryPlan key={entry.id} entry={entry} masterOn={auto.enabled}
        onEdit={() => setEditing({ id: entry.id, kind: 'retry' })} />)}
    </> : null}
    {entry && editing ? <CheckinMakeupEditor key={`${entry.id}:${editing.kind}`} entry={entry} kind={editing.kind}
      options={options} defaultRetryTime={auto.defaultRetryTime || '00:10'} onClose={close} /> : null}
  </div>
}
