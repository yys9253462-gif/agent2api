import { Badge, Button, Card, CardFooter } from '@ui'
import { apiEndpoints } from '../../../ui-shared/docs-api-endpoints'
import { DocumentationBrowser } from '../../../ui-shared/docs-api-browser'
import { useProviderDocumentation } from './DocsProviderData'
import { copyWithToast } from '../components/ui'
import { Icon } from '../components/Icon'
import { apiBase, serverIdentity } from '../lib/servers'
import { t } from '../i18n'

function ApiAddresses({ origin }: { origin: string }) {
  const endpoints = apiEndpoints()

  return <Card className='m-card docs-addresses'>
    {endpoints.map((item, index) => {
      const url = `${origin}${item.path}`
      return <div key={item.id}>
        {item.group && item.group !== endpoints[index - 1]?.group ? <div
          className='border-b border-border bg-surface-inset px-3.5 py-2 text-[11px] font-semibold text-muted-foreground'>
          {item.group}
        </div> : null}
        <div className='flex items-center gap-2.5 border-b border-hairline px-3.5 py-3'>
          <div className='min-w-0 flex-1'>
            <div className='flex flex-wrap items-center gap-2 text-[12.5px] font-medium'>
              {item.method ? <Badge variant='outline' className='font-mono text-[10px]'>{item.method}</Badge> : null}
              <span>{item.name}</span>
            </div>
            <code className='mt-1 block break-all font-mono text-[11.3px] leading-relaxed text-subtle'>{url}</code>
            <div className='mt-1 text-[11px] leading-relaxed text-muted-foreground'>{item.note}</div>
          </div>
          <Button size='icon-touch' variant='ghost' type='button' className='shrink-0 text-primary'
            onClick={() => void copyWithToast(url, item.name)} aria-label={t('复制地址')}>
            <Icon name='copy' style={{ width: 16, height: 16 }} />
          </Button>
        </div>
      </div>
    })}
    <CardFooter className='flex-col items-start gap-1.5'>
      <span>{t('生成、编辑与对话接口共用网关 Key；鉴权规则、模型能力和请求示例可从文档目录查阅。')}</span>
      <span>{t('Anthropic 官方 SDK / Claude Code 使用的根地址：')}<code className='break-all'>{origin}</code></span>
    </CardFooter>
  </Card>
}

export function DocsPage() {
  const origin = apiBase()
  const { packs, error, loading, refresh } = useProviderDocumentation()

  return <div className='m-page m-fixed docs-mobile-page'>
    <DocumentationBrowser key={serverIdentity()} origin={origin} packs={packs}
      addresses={<ApiAddresses origin={origin} />} onCopy={copyWithToast}
      notice={error ? <div role='status' className='mb-3 flex flex-wrap items-center gap-2 text-[12px] text-subtle'>
        <span>{t('读取提供商能力表失败：{error}', { error })}</span>
        <Button variant='outline' size='xs' disabled={loading} onClick={refresh}>{t('重试')}</Button>
      </div> : null} />
  </div>
}
