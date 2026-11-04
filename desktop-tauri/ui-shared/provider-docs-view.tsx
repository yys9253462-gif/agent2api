import { Badge, Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@ui'
import { t } from '../ui-kit/src/i18n'
import { documentText, PROVIDER_CAPABILITY_KEYS, type PluginProviderDocumentation,
  type ProviderCapabilityKey, type ProviderCapabilityStatus, type ProviderDocumentation } from './provider-docs'

function columns(): Record<ProviderCapabilityKey, string> {
  return {
    chat: t('LLM 请求'),
    refresh: t('Token 自动续期'),
    model_refresh: t('在线模型列表'),
    usage: t('余额查询'),
    checkin: t('签到'),
    claims: t('领取类'),
  }
}

function statusLabel(status: ProviderCapabilityStatus): string {
  switch (status) {
    case 'supported': return t('支持')
    case 'implemented_unverified': return t('已实现，待实测')
    case 'unsupported': return t('不支持')
    case 'not_applicable': return t('无此概念')
    default: return t('未声明')
  }
}

function Status({ status }: { status: ProviderCapabilityStatus }) {
  const symbols: Record<ProviderCapabilityStatus, string> = {
    supported: '✓', implemented_unverified: t('待实测'), unsupported: '✗', not_applicable: '—', undeclared: '?',
  }
  return <span aria-label={statusLabel(status)} title={statusLabel(status)}
    className={status === 'supported' ? 'text-success font-semibold' : 'text-muted-foreground'}>
    {symbols[status] || '?'}
  </span>
}

function rowNotes(provider: ProviderDocumentation, locale: string, labels: Record<ProviderCapabilityKey, string>): string[] {
  const notes = provider.notes.map(note => documentText(note, locale)).filter(Boolean)
  for (const key of PROVIDER_CAPABILITY_KEYS) {
    const note = documentText(provider.capabilities[key]?.note, locale)
    if (note) notes.push(`${labels[key]}：${note}`)
  }
  return [...new Set(notes)]
}

function packTitle(pack: PluginProviderDocumentation): string {
  return pack.id === 'builtin-subscriptions' ? t('本体订阅') : t(pack.name)
}

export function ProviderDocsTable({ packs }: { packs: PluginProviderDocumentation[] }) {
  const locale = (window as unknown as { wbI18n?: { locale(): string } }).wbI18n?.locale() || 'zh-Hans'
  const labels = columns()
  return <div className='flex min-w-0 flex-col gap-3'>
    {packs.map(pack => <Card key={pack.id}>
      <CardHeader>
        <CardTitle>{packTitle(pack)}</CardTitle>
        <Badge variant='outline'>v{pack.version}</Badge>
        {pack.description ? <CardDescription className='w-full'>
          {documentText(pack.description, locale)}
        </CardDescription> : null}
      </CardHeader>
      <CardContent className='p-0'>
        <Table className='min-w-[860px]' aria-label={`${packTitle(pack)} ${t('提供商能力适配表')}`}>
          <TableHeader><TableRow>
            <TableHead className='min-w-[170px]'>{t('提供商')}</TableHead>
            {PROVIDER_CAPABILITY_KEYS.map(key => <TableHead key={key} className='text-center'>
              {labels[key]}
            </TableHead>)}
            <TableHead className='min-w-[250px]'>{t('说明')}</TableHead>
          </TableRow></TableHeader>
          <TableBody>{pack.providers.map(provider => {
            const notes = rowNotes(provider, locale, labels)
            return <TableRow key={provider.id}>
              <TableCell>
                <div className='flex flex-col items-start gap-1'>
                  <span className='font-medium'>{t(provider.label)}</span>
                  {!provider.enabled ? <Badge variant='outline'>{t('已停用')}</Badge>
                    : !provider.loaded ? <Badge variant='outline'>{t('未加载')}</Badge> : null}
                </div>
              </TableCell>
              {PROVIDER_CAPABILITY_KEYS.map(key => <TableCell key={key} className='text-center'>
                <Status status={provider.capabilities[key]?.status || 'undeclared'} />
              </TableCell>)}
              <TableCell className='max-w-[380px] text-[11.5px] leading-relaxed text-subtle'>
                {notes.length ? <div className='flex flex-col gap-1'>
                  {notes.map(note => <span key={note}>{note}</span>)}
                </div> : '—'}
              </TableCell>
            </TableRow>
          })}</TableBody>
        </Table>
      </CardContent>
      <CardFooter className='flex-col items-start gap-1'>
        <span>{t('✓ 支持 · ✗ 不支持 · — 无此概念 · ? 未声明')}</span>
        <span>{t('已实现，待实测：仅表示代码已接入，尚未通过真实账号验证。')}</span>
        <span>{t('在线模型列表指自动拉取上游目录；仅有静态模型清单不算在线支持。')}</span>
        <span>{t('未声明表示网关本体或插件没有提供明确结论，不等于不支持。')}</span>
      </CardFooter>
    </Card>)}
  </div>
}
