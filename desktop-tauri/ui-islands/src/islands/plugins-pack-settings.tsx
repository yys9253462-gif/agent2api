import * as React from 'react'
import {
  Badge, Button, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter,
  DialogHeader, DialogTitle, Input, Label, Switch,
} from '@ui'
import { pluginName } from './plugins-shared'
import type { PackProvider, PackSummary } from './plugins-types'
import { t } from '../i18n'

export function PluginPackSettingsDialog({ pack, name, disabled, onClose, onEnabled, onProvider }: {
  pack: PackSummary
  name?: string
  disabled: boolean
  onClose: () => void
  onEnabled: (enabled: boolean) => void
  onProvider: (provider: PackProvider, enabled: boolean) => void
}) {
  const [search, setSearch] = React.useState('')
  const idPrefix = React.useId()
  const keyword = search.trim().toLocaleLowerCase()
  const providers = pack.providers.filter(provider => !keyword
    || `${provider.label} ${provider.id}`.toLocaleLowerCase().includes(keyword))

  return <Dialog open onOpenChange={next => { if (!next) onClose() }}>
    <DialogContent>
      <DialogHeader>
        <DialogTitle className='min-w-0 break-words'>{pluginName(pack.id, name || pack.name)} · {t('设置')}</DialogTitle>
      </DialogHeader>
      <DialogBody className='min-h-0 gap-3.5'>
        <div className='flex flex-wrap items-center justify-between gap-3'>
          <div className='flex flex-wrap items-center gap-2 text-[12px] text-muted-foreground'>
            <span className='tabular-nums'>v{pack.version}</span>
            <Badge variant='outline'>{t('{count} 家', { count: pack.providers.length })}</Badge>
          </div>
          <Label htmlFor={`${idPrefix}-enabled`} className='text-[12px] text-subtle'>
            {t('启用')}
            <Switch id={`${idPrefix}-enabled`} checked={pack.enabled} disabled={disabled}
              onCheckedChange={value => onEnabled(value === true)} />
          </Label>
        </div>
        <DialogDescription className='text-[12px] leading-relaxed text-subtle'>
          {t('这个包提供的提供商（可单独停用某一家）')}
        </DialogDescription>
        {!pack.enabled ? <Badge variant='secondary' className='self-start'>{t('已停用')}</Badge> : null}
        {pack.providers.length ? <>
          <Input type='search' value={search} placeholder={t('搜索提供商')} aria-label={t('搜索提供商')}
            onChange={event => setSearch(event.currentTarget.value)} />
          <div className='max-h-[46vh] min-h-0 overflow-y-auto overscroll-contain rounded-md border border-hairline'>
            {providers.length ? <div className='grid grid-cols-1 gap-2 p-2 min-[640px]:grid-cols-2'>
              {providers.map(provider => <div key={provider.id}
                className='flex min-w-0 items-center gap-3 rounded-md border border-hairline bg-surface-2 px-3 py-2.5'>
                <Label htmlFor={`${idPrefix}-${provider.id}`} className='min-w-0 flex-1 flex-col items-start gap-0.5'>
                  <span className='text-[12.5px] font-medium break-words text-foreground'>{t(provider.label)}</span>
                  <code className='max-w-full text-[11px] break-all text-muted-foreground'>{provider.id}</code>
                  {provider.enabled && !provider.loaded ? <Badge variant='secondary' className='mt-1'>{t('未加载')}</Badge> : null}
                </Label>
                <Switch id={`${idPrefix}-${provider.id}`} checked={provider.enabled} disabled={disabled || !pack.enabled}
                  onCheckedChange={value => onProvider(provider, value === true)} />
              </div>)}
            </div> : <p className='px-3 py-5 text-center text-[12px] text-subtle'>
              {t('没有匹配「{keyword}」的提供商', { keyword: search.trim() })}
            </p>}
          </div>
        </> : null}
      </DialogBody>
      <DialogFooter className='justify-end'>
        <Button variant='outline' size='sm' onClick={onClose}>{t('关闭')}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
}
