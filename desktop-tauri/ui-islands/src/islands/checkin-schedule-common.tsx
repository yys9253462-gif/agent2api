import * as React from 'react'
import { Badge, Button, Card, CardContent, CardHeader, Switch } from '@ui'
import { t } from '../i18n'
import { store, toggleMaster } from './checkin-store'
import type { AutoState } from './checkin-types'
import { beijingScheduleTime, goCheckinTab } from './checkin-schedule-model'

export function CheckinScheduleMaster({ auto }: { auto: AutoState }): React.ReactElement {
  const enabledCount = (auto.schedules ?? []).filter(entry => entry.enabled).length
  const next = (auto.schedules ?? []).filter(entry => entry.enabled)
    .map(entry => Number(entry.nextRunAt)).filter(value => Number.isFinite(value) && value > 0)
    .sort((a, b) => a - b)[0]
  return (
    <Card render={<section />} className='ck-schedule-master'>
      <CardContent className='ck-schedule-master-content flex items-center'>
        <div className='ck-schedule-master-copy'>
          <div className='ck-schedule-title'>
            <h2>{t('自动执行总开关')}</h2>
            <Badge variant={auto.enabled ? 'success' : 'outline'} shape='tag'>
              {auto.enabled ? t('已开启') : t('已暂停')}
            </Badge>
          </div>
          <p>{t('同时控制定时签到、定时补签和活动窗口重试；猫猫旅行独立运行。')}</p>
        </div>
        <div className='ck-schedule-next'>
          <span>{auto.enabled ? t('下次自动执行') : t('自动执行已暂停')}</span>
          <strong>{auto.enabled && next ? beijingScheduleTime(next) : '—'}</strong>
          <small>{t('北京时间 · 已启用 {n} 个签到计划', { n: enabledCount })}</small>
        </div>
        <Switch checked={auto.enabled} disabled={store.saving}
          aria-label={t('自动执行总开关')}
          onCheckedChange={enabled => void toggleMaster(enabled === true)} />
      </CardContent>
      {!auto.enabled ? <div className='ck-schedule-paused'>
        {t('签到与补签计划已暂停，配置仍保留；手动执行不受总开关影响。')}
      </div> : null}
    </Card>
  )
}

export function CheckinScheduleHeading({ title, description, action }: {
  title: string
  description: string
  action?: React.ReactNode
}): React.ReactElement {
  return <div className='ck-schedule-heading'>
    <div><h2>{title}</h2><p>{description}</p></div>{action}
  </div>
}

export function CheckinScheduleRules(): React.ReactElement {
  return <Card render={<section />} className='ck-schedule-rules'>
    <CardHeader><h3>{t('执行规则')}</h3></CardHeader>
    <CardContent>
      <p>{t('所有时间按北京时间计算。每个账号每天随机安排一次，重启保留当天安排；错过区间时会补跑未执行账号。')}</p>
      <p>{t('主签到已执行不等于每个账号都已领取成功；账号结果请查看签到记录。')}</p>
      <Button size='sm' variant='link' onClick={() => goCheckinTab('records')}>{t('查看签到记录')}</Button>
    </CardContent>
  </Card>
}
