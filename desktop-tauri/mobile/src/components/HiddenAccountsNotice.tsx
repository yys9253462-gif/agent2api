import { Button , Card } from '@ui'
import { Badge } from './ui'
import type { HiddenAccountReason, HiddenAccountsSummary } from '../../../ui-shared/plugins'
import { store } from '../state/store'
import { t } from '../i18n'

function reasonLabel(reason: HiddenAccountReason): string {
  switch (reason) {
    case 'not_installed': return t('插件未安装')
    case 'pack_disabled': return t('插件包已停用')
    case 'provider_disabled': return t('提供商已停用')
    case 'not_loaded': return t('插件未加载')
    default: return t('插件未就绪')
  }
}

export function HiddenAccountsNotice({ hidden }: { hidden?: HiddenAccountsSummary }) {
  if (!hidden?.total) return null
  return (
    <div className='m-section'>
      <Card className='m-card m-card-pad' role='status' style={{ background: 'var(--m-warn-soft)' }}>
        <p style={{ fontSize: 13, fontWeight: 600, lineHeight: 1.6, margin: '0 0 8px' }}>
          {t('有 {n} 个账号因提供商插件未就绪，暂未显示', { n: hidden.total })}
        </p>
        {hidden.providers?.map(provider => <div key={provider.id}
          style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 8, marginBottom: 6, fontSize: 12 }}>
          <span>{t(provider.label)} ×{provider.count}</span><Badge tone='warn'>{reasonLabel(provider.reason)}</Badge>
        </div>)}
        <p style={{ fontSize: 12, color: 'var(--m-text-2)', lineHeight: 1.7 }}>
          {t('账号数据仍然保留，无需重新添加。安装或启用对应插件并成功加载后，账号会自动恢复显示。')}
        </p>
        <Button variant='outline' size='sm' onClick={() => store.push('plugins')}>{t('查看插件状态')}</Button>
      </Card>
    </div>
  )
}
