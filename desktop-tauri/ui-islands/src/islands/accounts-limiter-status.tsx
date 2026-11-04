import * as React from 'react'
import { Badge, Button } from '@ui'
import { t } from '../i18n'
import type { AccountRecord } from './accounts-shared'
import { groupsOf, limiterHit, statesOf } from './accounts-limiter-model'

export function LimiterStatus({ account, onOpen }: { account: AccountRecord; onOpen: () => void }) {
  if (account.enabled === false || !limiterHit(account)) return null
  const states = statesOf(account)
  const groups = groupsOf(account)
  const all = groups.some((group, index) => group.action === 'skip' && states[index]?.hit && states[index]?.account)
  const models = [...new Set(groups.flatMap((group, index) => group.action === 'skip' && states[index]?.hit ? states[index].models : []))]
  const label = all || !models.length ? t('限制器 · 账号已跳过') : t('限制器 · {n} 个模型', { n: models.length })
  return <Button variant='ghost' size='xs' className='h-auto p-0' onClick={onOpen}
    title={all ? t('账号级条件组已触发；点击查看限制器') : t('仅跳过命中条件的模型，其他模型仍可转发；点击查看限制器')}>
    <Badge variant='warning' shape='tag'>{label}</Badge>
  </Button>
}
