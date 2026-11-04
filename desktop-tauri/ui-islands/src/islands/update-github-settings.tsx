import * as React from 'react'
import { Badge, Button, Input, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, cn } from '@ui'
import {
  buildProxyPick, EMPTY_PROXY_SELECTION, errorMessage, fetchProxySelection, openExternal,
  saveProxy, shared, toast, type ProxySelection, type UpdateTokenStatus,
} from './update-shared'
import { GithubAccessSettingsPanel } from './update-github-access'
import { t } from '../i18n'

function tokenStatus(token: UpdateTokenStatus | null): { badge: React.ReactNode; hint: string } {
  if (!token) return { badge: <Badge shape='tag' variant='outline'>{t('读取中')}</Badge>, hint: '' }
  if (token.error) return {
    badge: <Badge shape='tag' variant='destructive' title={token.error}>{t('无法解密')}</Badge>,
    hint: t('重新粘贴保存一次即可自愈'),
  }
  if (token.filled && token.origin === 'stored') return {
    badge: <Badge shape='tag' variant='success'>{t('已填写')}</Badge>,
    hint: t('已加密存储在本地，不显示具体值；粘贴新令牌可覆盖'),
  }
  if (token.filled) return {
    badge: <Badge shape='tag' variant='secondary'>{t('环境变量')}</Badge>,
    hint: t('已配置 GITHUB_TOKEN 环境变量，界面保存的令牌优先于它'),
  }
  return {
    badge: <Badge shape='tag' variant='outline'>{t('未填写')}</Badge>,
    hint: t('填写后检查限额 60 → 5000 次/小时（创建令牌无需勾选任何权限）'),
  }
}

export function GithubUpdateSettings({ plugin = false, pane }: {
  plugin?: boolean
  pane: 'network' | 'token' | null
}) {
  const [selection, setSelection] = React.useState<ProxySelection>(EMPTY_PROXY_SELECTION)
  const [token, setToken] = React.useState<UpdateTokenStatus | null>(null)
  const [draft, setDraft] = React.useState('')
  const [busy, setBusy] = React.useState(false)
  const [proxyBusy, setProxyBusy] = React.useState(false)
  const proxyRevision = React.useRef(0)
  const pick = buildProxyPick(selection)

  React.useEffect(() => {
    let alive = true
    const revision = proxyRevision.current
    void fetchProxySelection().then(value => {
      if (alive && revision === proxyRevision.current) setSelection(value)
    })
    void shared().workbuddyDesktop?.getUpdateToken().then(value => {
      if (alive) setToken(value ?? null)
    }).catch(error => toast(t('读取设置失败：{error}', { error: errorMessage(error) }), 'err'))
    return () => { alive = false }
  }, [])

  async function handlePick(value: string): Promise<void> {
    if (proxyBusy) return
    proxyRevision.current += 1
    setProxyBusy(true)
    try {
      const result = await saveProxy(value)
      if (result) {
        setSelection(previous => ({ ...previous, proxyChoice: result.choice }))
        const next = await fetchProxySelection()
        setSelection(previous => ({
          ...next,
          proxyChoice: next.proxyLoaded ? next.proxyChoice : result.choice,
          pool: next.poolError ? previous.pool : next.pool,
        }))
      }
    } finally { setProxyBusy(false) }
  }

  async function saveToken(clear = false): Promise<void> {
    if (busy) return
    const value = draft.trim()
    if (!clear && !value) return
    setBusy(true)
    try {
      const result = await shared().workbuddyDesktop?.setUpdateToken({ token: clear ? null : value })
      if (!result) throw new Error(t('请求失败'))
      setToken(result)
      setDraft('')
      if (result.saved === false) toast(t('令牌已生效，但写入磁盘失败（重启后会丢失）'), 'err')
      else toast(clear
        ? t('✅ 已清除界面保存的 GitHub 令牌（环境变量若配置过则继续生效）')
        : t('✅ GitHub 令牌已保存（加密存储，界面不再显示）'))
    } catch (error) {
      toast(t('保存失败：{error}', { error: errorMessage(error) }), 'err')
    } finally { setBusy(false) }
  }

  const status = tokenStatus(token)
  const stored = Boolean(token?.error) || token?.origin === 'stored'
  return (
    <>
      <div hidden={pane !== 'network'}>
        <div className='settings-dlg-pane-head'>
          <h3>{t('网络')}</h3>
          {plugin ? <p>{t('与软件更新共用出网代理和 GitHub 凭证，修改任意一处都会同步生效。')}</p> : null}
        </div>
        <div className='settings-dlg-fields'>
          <GithubAccessSettingsPanel />
          <div className='h-px bg-hairline' />
          <div>
            <div className='text-[13px] font-semibold text-foreground'>{t('出网代理')}</div>
            <div className='mt-1.5'>
              <Select value={pick.current} disabled={proxyBusy}
                onValueChange={value => void handlePick(String(value))}>
                <SelectTrigger className={cn('w-full', pick.broken && 'border-destructive-bd')}
                  title={pick.title} aria-label={t('更新出网代理')}>
                  <SelectValue className='min-w-0 truncate'>{pick.selected?.label || t('直连')}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {pick.items.map(item => (
                    <SelectItem key={item.value} value={item.value} disabled={item.disabled}>{item.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>
      </div>
      <div hidden={pane !== 'token'}>
        <div className='settings-dlg-pane-head'>
          <h3>{t('GitHub 令牌')}</h3>
          {plugin ? <p>{t('与软件更新共用出网代理和 GitHub 凭证，修改任意一处都会同步生效。')}</p> : null}
        </div>
        <div className='settings-dlg-fields'>
          <div className='flex flex-wrap items-center gap-2'>
            {status.badge}<span className='text-[12px] text-subtle'>{status.hint}</span>
          </div>
          <div className='flex flex-wrap items-center gap-2'>
            <Input type='password' className='min-w-0 flex-1' autoComplete='new-password' spellCheck={false}
              aria-label={t('GitHub 令牌')}
              placeholder={token?.filled && token.origin !== 'env'
                ? t('已填写（粘贴新令牌可覆盖）') : t('粘贴 GitHub 令牌（ghp_… / github_pat_…）')}
              value={draft} onChange={event => setDraft(event.currentTarget.value)}
              onKeyDown={event => {
                if (event.key === 'Enter') { event.preventDefault(); void saveToken() }
              }} />
            <Button size='sm' disabled={busy || !draft.trim()} onClick={() => void saveToken()}>{t('保存')}</Button>
            {stored ? <Button variant='outline' size='sm' disabled={busy} onClick={() => void saveToken(true)}>
              {t('清除')}
            </Button> : null}
          </div>
          <div>
            <Button variant='outline' size='sm'
              title={t('在默认浏览器中打开 GitHub 的令牌创建页（备注已预设，无需勾选任何权限，直接点 Generate token）')}
              onClick={() => void openExternal('https://github.com/settings/tokens/new?description='
                + encodeURIComponent('Agent2API 更新检查'))}>
              {t('打开 GitHub 令牌页面')}
            </Button>
          </div>
        </div>
      </div>
    </>
  )
}
