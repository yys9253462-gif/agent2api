import { useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Alert, Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogBody, DialogFooter } from '@ui'
import { callJson } from './assistant-state'
import { shared, type UpdateInfo } from './update-shared'
import { t } from '../i18n'

type PolicyResult = Pick<UpdateInfo, 'forceUpdate' | 'currentVersion' | 'repository'>

function RequiredUpdate() {
  const [policy, setPolicy] = useState<PolicyResult | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [onUpdatePage, setOnUpdatePage] = useState(shared().wbApp?.currentPage === 'settings')
  useEffect(() => {
    const refresh = () => setOnUpdatePage(shared().wbApp?.currentPage === 'settings')
    const observer = new MutationObserver(refresh)
    document.querySelectorAll('.page').forEach(page => observer.observe(page, { attributes: true, attributeFilter: ['class'] }))
    refresh()
    return () => observer.disconnect()
  }, [])
  useEffect(() => {
    let disposed = false
    const load = () => { void callJson<PolicyResult>('GET', '/api/update/policy').then(result => { if (!disposed) setPolicy(result) }).catch(() => {}) }
    load()
    const timer = setInterval(load, 60_000)
    return () => { disposed = true; clearInterval(timer) }
  }, [])
  const required = policy?.forceUpdate
  const active = required?.required === true
  const [updating, setUpdating] = useState(false)
  if (!active || (updating && onUpdatePage)) return null
  return <Dialog open onOpenChange={() => {}}>
    <DialogContent showCloseButton={false}>
      <DialogHeader><DialogTitle>{t('需要强制更新')}</DialogTitle></DialogHeader>
      <DialogBody>
        <Alert variant='warning'>{required.message}</Alert>
        <p>{t('当前版本 {current}，最低可用版本 {minimum}。', { current: policy?.currentVersion || '', minimum: required.minimumVersion })}</p>
        <p className='text-xs text-muted-foreground'>{t('更新前无法继续调用模型；账号、配置和历史记录不会被删除。')}</p>
        {error ? <Alert variant='destructive'>{error}</Alert> : null}
      </DialogBody>
      <DialogFooter>
        <Button variant='outline' disabled={busy} onClick={() => {
          shared().wbApp?.showPage?.('settings'); shared().wbSettingsPanel?.showCategory?.('about'); setUpdating(true)
        }}>{t('更新设置')}</Button>
        <Button disabled={busy} onClick={async () => {
          setBusy(true); setError('')
          try {
            const info = await shared().workbuddyDesktop?.checkUpdate()
            if (!info) throw new Error(t('无法读取更新信息'))
            if (!info.forceUpdate?.required) { setPolicy(info); return }
            if (!window.wbUpdatePanel?.openAndDownload) throw new Error(t('软件版本检查设置不可用'))
            shared().wbApp?.showPage?.('settings'); shared().wbSettingsPanel?.showCategory?.('about'); setUpdating(true)
            await window.wbUpdatePanel.openAndDownload(info)
          } catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)) }
          finally { setBusy(false) }
        }}>{t('立即更新')}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
}

let mounted = false
function mount() {
  if (mounted) return
  mounted = true
  const host = document.createElement('div')
  document.body.append(host)
  createRoot(host).render(<RequiredUpdate />)
}
if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', mount, { once: true })
else mount()
