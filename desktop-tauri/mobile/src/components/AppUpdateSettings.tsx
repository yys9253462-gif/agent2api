import { useEffect, useState } from 'react'
import { Button, Input, Switch } from '@ui'
import { getUpdateChannels, getUpdateCheckPlan, saveUpdateChannels, saveUpdateCheckPlan, type UpdateChannels, type UpdateCheckPlan } from '../api/updates'
import { serverIdentity } from '../lib/servers'
import { store } from '../state/store'
import { t } from '../i18n'
import { Alert } from '@ui'

export function AppUpdateSettings({ onChanged }: { onChanged: (channelsChanged: boolean) => void }) {
  const [channels, setChannels] = useState<UpdateChannels | null>(null)
  const [plan, setPlan] = useState<UpdateCheckPlan | null>(null)
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const identity = serverIdentity()

  useEffect(() => {
    const controller = new AbortController()
    void getUpdateChannels(controller.signal).then(value => {
      if (!controller.signal.aborted) setChannels(value)
    }).catch(cause => { if (!controller.signal.aborted) setError(String(cause)) })
    void getUpdateCheckPlan(controller.signal).then(value => {
      if (!controller.signal.aborted) { setPlan(value); setDraft(String(value.interval)) }
    }).catch(cause => { if (!controller.signal.aborted) setError(String(cause)) })
    return () => controller.abort()
  }, [])

  async function saveChannel(patch: Partial<Pick<UpdateChannels, 'stable' | 'beta'>>) {
    if (busy || !channels) return
    const next = { ...channels, ...patch }
    if (!next.stable && !next.beta) {
      setError(t('至少需要关注正式版或 beta 版中的一个通道'))
      return
    }
    setBusy(true)
    setError('')
    try {
      const result = await saveUpdateChannels(patch)
      if (serverIdentity() !== identity) return
      setChannels(result)
      store.toast(result.saved === false ? t('通道已切换，但写入磁盘失败（重启后会回到原设置）') : t('更新通道已保存'), result.saved === false ? 'err' : 'ok')
      onChanged(true)
    } catch (cause) {
      if (serverIdentity() === identity) setError(cause instanceof Error ? cause.message : String(cause))
    } finally { setBusy(false) }
  }

  async function savePlan(patch: Partial<Pick<UpdateCheckPlan, 'enabled' | 'interval'>>) {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      const result = await saveUpdateCheckPlan(patch)
      if (serverIdentity() !== identity) return
      setPlan(result)
      setDraft(String(result.interval))
      store.toast(result.saved === false ? t('设置已生效，但写入磁盘失败（重启后会恢复原设置）') : t('✅ 自动检查更新已保存'), result.saved === false ? 'err' : 'ok')
      onChanged(false)
    } catch (cause) {
      if (serverIdentity() === identity) setError(cause instanceof Error ? cause.message : String(cause))
    } finally { setBusy(false) }
  }

  function saveInterval() {
    const interval = Number(draft)
    if (!Number.isInteger(interval) || interval < 1 || interval > 1440) {
      setError(t('检查间隔必须是 {min} ~ {max} 分钟', { min: 1, max: 1440 }))
      return
    }
    void savePlan({ interval })
  }

  return <div className='flex flex-col gap-2 px-3 py-3' style={{ borderTop: '1px solid var(--m-line)' }}>
    <div className='m-t1'>{t('更新设置')}</div>
    <p className='m-t2'>{t('以下设置属于所连接网关，不是手机本地设置。')}</p>
    <div className='flex items-center justify-between gap-2'>
      <span className='m-t1'>{t('自动检查更新')}</span>
      <Switch checked={plan?.enabled === true} disabled={busy || !plan} aria-label={t('自动检查更新')}
        onCheckedChange={value => void savePlan({ enabled: value === true })} />
    </div>
    <div className='flex items-center gap-2'>
      <Input type='number' min={1} max={1440} step={1} className='w-[110px] text-base' value={draft} disabled={busy || !plan?.enabled}
        onChange={event => setDraft(event.currentTarget.value)} aria-label={t('检查间隔（分钟）')} />
      <span className='m-t2'>{t('分钟（{min} ~ {max}）', { min: 1, max: 1440 })}</span>
      <Button size='sm' variant='outline' disabled={busy || !plan?.enabled || Number(draft) === plan.interval} onClick={saveInterval}>{t('保存')}</Button>
    </div>
    <div className='flex items-center justify-between gap-2'>
      <span className='m-t1'>{t('关注正式版更新')}</span>
      <Switch checked={channels?.stable === true} disabled={busy || !channels} aria-label={t('关注正式版更新')}
        onCheckedChange={value => void saveChannel({ stable: value === true })} />
    </div>
    <p className='m-t2'>{t('默认开启；即使当前使用 beta 版，也可收到版本号更高的正式版更新。')}</p>
    <div className='flex items-center justify-between gap-2'>
      <span className='m-t1'>{t('关注 beta 版更新')}</span>
      <Switch checked={channels?.beta === true} disabled={busy || !channels} aria-label={t('关注 beta 版更新')}
        onCheckedChange={value => void saveChannel({ beta: value === true })} />
    </div>
    <p className='m-t2'>{t('beta 版可能不稳定。两项同时开启时，按版本号选择最高版本，不会降级；安装到正式版或 beta 版后仍保留关注选择。')}</p>
    <p className='m-t2'>{t('插件默认跟随这里的更新通道、自动检查和间隔；可在插件设置中单独设置。')}</p>
    {error ? <Alert variant='destructive' className='m-t2' role='alert' style={{ color: 'var(--m-danger)' }}>{error}</Alert> : null}
  </div>
}
