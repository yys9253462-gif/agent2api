import { useEffect, useState } from 'react'
import { Button, Input, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@ui'
import { apiCall } from '../api/client'
import { store } from '../state/store'
import { t } from '../i18n'
import { Alert } from '@ui'

type GithubAccess = {
  mode: 'auto' | 'official' | 'accelerated'
  mirrorUrl: string
  saved?: boolean
}

export function GithubAccessSettingsPanel() {
  const [settings, setSettings] = useState<GithubAccess | null>(null)
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const options = [
    { value: 'auto', label: t('自动（官方优先）') },
    { value: 'official', label: t('仅官方源') },
    { value: 'accelerated', label: t('加速源优先') },
  ] as const

  useEffect(() => {
    let alive = true
    void apiCall<GithubAccess>('/api/update/github').then(value => {
      if (alive) { setSettings(value); setDraft(value.mirrorUrl) }
    }).catch(failure => { if (alive) setError(failure instanceof Error ? failure.message : String(failure)) })
    return () => { alive = false }
  }, [])

  async function save(patch: Partial<Pick<GithubAccess, 'mode' | 'mirrorUrl'>>): Promise<void> {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      const value = await apiCall<GithubAccess>('/api/update/github', { method: 'POST', body: patch })
      setSettings(value)
      setDraft(value.mirrorUrl)
      store.toast(value.saved === false
        ? t('设置已生效，但写入磁盘失败（重启后会恢复原设置）') : t('GitHub 访问设置已保存'))
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure))
    } finally { setBusy(false) }
  }

  return (
    <div className='flex flex-col gap-2 px-3 py-3' style={{ borderTop: '1px solid var(--m-line)' }}>
      <div className='m-t1'>{t('GitHub 访问方式')}</div>
      <Select value={settings?.mode || 'auto'} disabled={busy || !settings}
        onValueChange={value => void save({ mode: value as GithubAccess['mode'] })}>
        <SelectTrigger className='w-full' aria-label={t('GitHub 访问方式')}>
          <SelectValue>{options.find(option => option.value === settings?.mode)?.label || options[0].label}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          {options.map(option => <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>)}
        </SelectContent>
      </Select>
      <p className='m-t2'>
        {settings?.mode === 'official' ? t('只访问 GitHub 官方源，仍使用所选出网代理。')
          : settings?.mode === 'accelerated' ? t('优先使用加速源，访问失败后回退官方源；不会绕过 GitHub 限流冷却。')
            : t('优先访问官方源，连接失败后尝试加速源；短期记住成功的源，定期恢复官方优先。')}
      </p>
      <div className='m-t2'>{t('GitHub 加速前缀')}</div>
      <div className='flex items-center gap-2'>
        <Input className='min-w-0 flex-1' value={draft} autoComplete='off' spellCheck={false}
          placeholder='https://gh-proxy.com/' aria-label={t('GitHub 加速前缀')}
          disabled={busy || !settings || settings.mode === 'official'}
          onChange={event => setDraft(event.currentTarget.value)} />
        <Button size='sm' variant='outline'
          disabled={busy || !settings || settings.mode === 'official' || draft === settings.mirrorUrl}
          onClick={() => void save({ mirrorUrl: draft })}>{t('保存')}</Button>
      </div>
      <p className='m-t2'>{t('填写支持 Raw、Release 和 GitHub API 的 HTTPS 加速站前缀，留空恢复默认。')}</p>
      <p className='m-t2'>{t('与网关的软件更新和插件共用设置；APK 下载由手机联网，GitHub 令牌不会发送给加速站。')}</p>
      {error ? <Alert variant='destructive' className='m-t2' role='alert' style={{ color: 'var(--m-danger)' }}>{error}</Alert> : null}
    </div>
  )
}
