import * as React from 'react'
import { Button } from '@ui'
import { t } from '../i18n'
import { bridge, errorMessage, notify, type ReplayRow, type StickyEntry } from './routing-types'
import { loadWorkspace } from './routing-state'

export function ReplayStickyBinding({ row }: { row: ReplayRow }) {
  const [entry, setEntry] = React.useState<StickyEntry | null>(null)
  const [loaded, setLoaded] = React.useState(false)
  const [busy, setBusy] = React.useState(false)
  const [error, setError] = React.useState('')
  const id = row.route?.stickyId
  React.useEffect(() => {
    let alive = true
    setEntry(null)
    setLoaded(false)
    setError('')
    if (!id) return () => { alive = false }
    void bridge().getRoutingSticky(row.model).then(result => {
      if (!alive) return
      setEntry(result.entries.find(entry => entry.id === id) || null)
      setLoaded(true)
    }).catch(error => { if (alive) { setError(errorMessage(error)); setLoaded(true) } })
    return () => { alive = false }
  }, [id, row.model])
  if (!id) return null
  const clear = async () => {
    if (busy || !entry) return
    setBusy(true)
    try {
      const result = await bridge().clearRoutingSticky({ id: entry.id, model: entry.model })
      setEntry(null)
      notify(t('已解除 {n} 条会话粘滞', { n: result.cleared }))
      await loadWorkspace(true)
    } catch (error) { setError(errorMessage(error)) }
    finally { setBusy(false) }
  }
  return <div className='rounded-md border border-hairline bg-surface-2 p-3 text-xs'>
    <div className='flex items-center gap-3'><span className='flex-1 text-muted-foreground'>{entry
      ? t('这条请求对应的会话当前仍绑定到「{account}」。', { account: entry.accountName })
      : loaded ? t('这条会话已解除或过期，不再有活跃绑定。') : t('正在读取会话绑定…')}</span>
      {entry ? <Button variant='outline' size='sm' disabled={busy} onClick={() => void clear()}>{t('解除这条会话')}</Button> : null}
    </div>{error ? <p role='alert' className='mt-2 text-destructive'>{error}</p> : null}
  </div>
}
