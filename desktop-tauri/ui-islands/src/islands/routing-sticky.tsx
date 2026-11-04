import * as React from 'react'
import {
  Badge, Button, Dialog, DialogBody, DialogContent, DialogFooter, DialogHeader, DialogTitle,
  Empty, Input, confirmDialog,
} from '@ui'
import { t } from '../i18n'
import { bridge, errorMessage, notify, type StickyEntry } from './routing-types'
import { loadWorkspace } from './routing-state'
import { formatDuration, RouteIcon, stickyLabel, timeText } from './routing-ui'

export function RoutingStickyDialog({ model, onClose }: { model: string; onClose(): void }) {
  const [entries, setEntries] = React.useState<StickyEntry[]>([])
  const [mode, setMode] = React.useState('')
  const [now, setNow] = React.useState(Date.now())
  const [offset, setOffset] = React.useState(0)
  const [search, setSearch] = React.useState('')
  const [busy, setBusy] = React.useState(false)
  const [error, setError] = React.useState('')
  const load = React.useCallback(async () => {
    try {
      const result = await bridge().getRoutingSticky(model || undefined)
      setEntries(result.entries)
      setMode(result.sticky)
      setOffset(result.now - Date.now())
      setNow(result.now)
      setError('')
    } catch (error) { setError(errorMessage(error)) }
  }, [model])
  React.useEffect(() => { void load() }, [load])
  React.useEffect(() => {
    const timer = window.setInterval(() => { if (!document.hidden) setNow(Date.now() + offset) }, 1000)
    return () => window.clearInterval(timer)
  }, [offset])
  const active = entries.filter(entry => entry.expiresAt > now)
  const visible = active.filter(entry => `${entry.model} ${entry.accountName} ${entry.providerLabel} ${entry.source} ${entry.keyPreview}`.toLowerCase().includes(search.toLowerCase()))
  const clear = async (entry?: StickyEntry) => {
    if (busy) return
    if (!entry && !await confirmDialog(model
      ? t('解除这个模型的全部会话粘滞。不会终止在途请求，之后的请求会重新选路。')
      : t('解除所有模型的全部会话粘滞。不会终止在途请求，之后的请求会重新选路。'), {
      title: t('清除会话粘滞'), confirmText: t('全部解除'),
    })) return
    setBusy(true)
    try {
      const result = await bridge().clearRoutingSticky(entry ? { id: entry.id, model: entry.model } : model ? { model } : { all: true })
      setEntries(current => entry ? current.filter(item => item.id !== entry.id) : [])
      notify(t('已解除 {n} 条会话粘滞', { n: result.cleared }))
      await load()
      await loadWorkspace(true)
    } catch (error) { setError(errorMessage(error)) }
    finally { setBusy(false) }
  }
  return <Dialog open onOpenChange={open => { if (!open && !busy) onClose() }}><DialogContent className='w-[min(820px,calc(100vw-48px))]'>
    <DialogHeader><RouteIcon name='route' /><DialogTitle>{t('会话粘滞')} · {model || t('全部模型')}</DialogTitle><Button variant='ghost' size='sm' disabled={busy} onClick={() => void load()}>{t('刷新')}</Button></DialogHeader>
    <DialogBody>
      <div className='flex flex-wrap items-center gap-2'><Badge variant='brand' shape='tag'>{t('粘性：{mode}', { mode: stickyLabel(mode) })}</Badge><span className='text-xs text-muted-foreground'>{t('{n} 条活跃绑定', { n: active.length })}</span></div>
      <p className='text-xs text-muted-foreground'>{t('解除只清除网关的账号绑定，不删除聊天历史、不关闭粘性设置。后续成功请求仍可建立新的绑定；重启网关会清空这些进程本地记录。')}</p>
      {mode === 'off' || mode === 'turn' ? <p className='rounded-md bg-warning-soft p-3 text-xs text-warning'>{t('当前档位不保留跨请求粘滞，因此没有活跃绑定。')}</p> : null}
      <Input placeholder={t('搜索模型、账号或会话来源')} value={search} onChange={event => setSearch(event.currentTarget.value)} />
      <div className='routing-sticky-list'>{visible.map(entry => <div className='routing-sticky-row' key={entry.id}>
        <span className='routing-sticky-mark'><RouteIcon name='route' size={20} /></span><div className='routing-sticky-info'><b>{entry.accountName}</b><span>{entry.providerLabel} · {entry.model}</span><div><Badge variant='secondary' shape='tag'>{entry.source === 'content-hash' ? t('内容哈希') : entry.source}</Badge><code>{entry.keyPreview}</code><span>{t('命中 {n} 次', { n: entry.hits })}</span></div><span>{t('最近使用：{time}', { time: timeText(entry.lastSeen) })}</span></div>
        <div className='routing-sticky-time'><b>{formatDuration(entry.expiresAt - now)}</b><span>{t('后过期')}</span></div><Button variant='outline' size='sm' disabled={busy} onClick={() => void clear(entry)}>{t('解除')}</Button>
      </div>)}
        {!visible.length ? <Empty>{search ? t('没有匹配的会话') : t('当前视图没有活跃会话粘滞')}</Empty> : null}
      </div>
      {error ? <p role='alert' className='text-xs text-destructive'>{error}</p> : null}
    </DialogBody><DialogFooter><Button variant='destructive' className='mr-auto' disabled={busy || !active.length} onClick={() => void clear()}>{model ? t('解除当前模型全部粘滞') : t('解除全部粘滞')}</Button><Button disabled={busy} onClick={onClose}>{t('完成')}</Button></DialogFooter>
  </DialogContent></Dialog>
}
