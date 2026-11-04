import * as React from 'react'
import { Badge, Button } from '@ui'
import { t } from '../i18n'
import { shared, type HiddenAccountReason, type HiddenAccountsSummary } from './accounts-shared'

function reasonLabel(reason: HiddenAccountReason): string {
  switch (reason) {
    case 'not_installed': return t('插件未安装')
    case 'pack_disabled': return t('插件包已停用')
    case 'provider_disabled': return t('提供商已停用')
    case 'not_loaded': return t('插件未加载')
  }
}

export function HiddenAccountsNotice({ hidden, compact = false }: {
  hidden: HiddenAccountsSummary
  compact?: boolean
}): React.ReactElement {
  const reasons = new Set(hidden.providers.map(provider => provider.reason))
  const installOnly = reasons.size === 1 && reasons.has('not_installed')
  const disabledOnly = reasons.size > 0 && [...reasons].every(reason =>
    reason === 'pack_disabled' || reason === 'provider_disabled')
  const action = installOnly ? t('去「插件」页安装')
    : disabledOnly ? t('去「插件」页启用') : t('查看插件状态')
  return (
    <div className={compact ? 'acct-hidden-banner' : 'empty acct-hidden-note'} role='status'>
      <div className='acct-hidden-main'>
        <p className='acct-hidden-title'>
          {compact
            ? t('另有 {n} 个账号因提供商插件未就绪，暂未显示', { n: hidden.total })
            : t('有 {n} 个账号因提供商插件未就绪，暂未显示', { n: hidden.total })}
        </p>
        {hidden.providers.length ? (
          <ul className='acct-hidden-list'>
            {hidden.providers.map(provider => (
              <li key={provider.id}>
                <span>{t('{label} ×{count}', { label: t(provider.label), count: provider.count })}</span>
                <Badge variant='warning' shape='tag'>{reasonLabel(provider.reason)}</Badge>
              </li>
            ))}
          </ul>
        ) : null}
        <p className='acct-hidden-foot'>
          {t('账号数据仍然保留，无需重新添加。安装或启用对应插件并成功加载后，账号会自动恢复显示。')}
        </p>
      </div>
      <Button size='sm' variant={compact ? 'outline' : 'default'}
        onClick={() => shared().wbApp?.showPage?.('plugins')}>
        {action}
      </Button>
    </div>
  )
}
