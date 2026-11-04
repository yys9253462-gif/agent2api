import * as React from 'react'
import { Button, Input, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@ui'
import { errorMessage, shared, toast, type GithubAccessSettings } from './update-shared'
import { t } from '../i18n'
import { Alert } from '@ui'

export function GithubAccessSettingsPanel() {
  const [settings, setSettings] = React.useState<GithubAccessSettings | null>(null)
  const [draft, setDraft] = React.useState('')
  const [busy, setBusy] = React.useState(false)
  const [error, setError] = React.useState('')
  const options = [
    { value: 'auto', label: t('自动（官方优先）') },
    { value: 'official', label: t('仅官方源') },
    { value: 'accelerated', label: t('加速源优先') },
  ] as const

  React.useEffect(() => {
    let alive = true
    void shared().workbuddyDesktop?.getGithubAccess().then(value => {
      if (!alive) return
      if (!value) throw new Error(t('读取设置失败：{error}', { error: t('请求失败') }))
      setSettings(value)
      setDraft(value.mirrorUrl)
    }).catch(failure => { if (alive) setError(errorMessage(failure)) })
    return () => { alive = false }
  }, [])

  async function save(patch: Partial<Pick<GithubAccessSettings, 'mode' | 'mirrorUrl'>>): Promise<void> {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      const result = await shared().workbuddyDesktop?.setGithubAccess(patch)
      if (!result) throw new Error(t('请求失败'))
      setSettings(result)
      setDraft(result.mirrorUrl)
      toast(result.saved === false
        ? t('设置已生效，但写入磁盘失败（重启后会恢复原设置）') : t('GitHub 访问设置已保存'),
      result.saved === false ? 'err' : 'ok')
    } catch (failure) {
      setError(errorMessage(failure))
    } finally { setBusy(false) }
  }

  return (
    <div className='flex flex-col gap-2'>
      <div className='text-[13px] font-semibold text-foreground'>{t('GitHub 访问方式')}</div>
      <Select value={settings?.mode || 'auto'} disabled={busy || !settings}
        onValueChange={value => void save({ mode: value as GithubAccessSettings['mode'] })}>
        <SelectTrigger className='w-full' aria-label={t('GitHub 访问方式')}>
          <SelectValue>{options.find(option => option.value === settings?.mode)?.label || options[0].label}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          {options.map(option => <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>)}
        </SelectContent>
      </Select>
      <p className='text-[12px] text-subtle'>
        {settings?.mode === 'official'
          ? t('只访问 GitHub 官方源，仍使用所选出网代理。')
          : settings?.mode === 'accelerated'
            ? t('优先使用加速源，访问失败后回退官方源；不会绕过 GitHub 限流冷却。')
            : t('优先访问官方源，连接失败后尝试加速源；短期记住成功的源，定期恢复官方优先。')}
      </p>
      <div className='text-[12px] font-medium text-foreground'>{t('GitHub 加速前缀')}</div>
      <div className='flex items-center gap-2'>
        <Input value={draft} disabled={busy || !settings || settings.mode === 'official'}
          autoComplete='off' spellCheck={false} className='min-w-0 flex-1'
          aria-label={t('GitHub 加速前缀')} placeholder='https://gh-proxy.com/'
          onChange={event => setDraft(event.currentTarget.value)}
          onKeyDown={event => {
            if (event.key === 'Enter') { event.preventDefault(); void save({ mirrorUrl: draft }) }
          }} />
        <Button size='sm' variant='outline' disabled={busy || !settings || settings.mode === 'official' || draft === settings.mirrorUrl}
          onClick={() => void save({ mirrorUrl: draft })}>{t('保存')}</Button>
      </div>
      <p className='text-[12px] text-subtle'>{t('填写支持 Raw、Release 和 GitHub API 的 HTTPS 加速站前缀，留空恢复默认。')}</p>
      <p className='text-[12px] text-subtle'>{t('软件更新与插件共用此设置；GitHub 令牌仅发送给官方 API。镜像安装包需要官方校验值，镜像插件包需要官方签名。')}</p>
      {error ? <Alert variant='destructive' role='alert' className='text-[12px] text-destructive'>{error}</Alert> : null}
    </div>
  )
}
