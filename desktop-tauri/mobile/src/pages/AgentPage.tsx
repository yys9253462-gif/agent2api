import { useState } from 'react'
import { PageIntro, IconButton, Badge, Empty, ErrorNote, SkeletonList } from '../components/ui'
import { GlyphLogo } from '../components/ProviderLogo'
import { Icon } from '../components/Icon'
import { PullIndicator, usePullToRefresh } from '../components/PullToRefresh'
import { store } from '../state/store'
import { fetchAgents, previewAgentRestore, restoreAgentClient, type AgentClient } from '../api/gateway'
import { usePolling } from '../lib/usePolling'
import { t } from '../i18n'
import { Card, Button, SegmentedControl } from '@ui'
import { AgentResourcesPanel } from '../../../ui-shared/agent-resources-panel'
import { apiCall } from '../api/client'
import type { ResourceKind, ResourceRequest } from '../../../ui-shared/agent-resources'
const resourceRequest: ResourceRequest = (method, path, body) => apiCall(path, { method, body })
import { confirmDialog } from '@ui'

/**
 * Agent 接管。
 *
 * ── 手机上只读，这是有意的 ────────────────────────────────────
 * 这一页的作用是改写**本机 AI 客户端的配置文件**（`~/.claude/settings.json`
 * 之类）。手机读不到那台机器的文件系统 —— 它连的是网关，而网关跑在另一台
 * 机器上。所以手机上：
 *   · 看得到有哪些客户端、各自的配置路径、是否已接管；
 *   · 可以「还原」（走网关的 `/api/agents/restore`，那是网关在替我们写）；
 *   · **不能**改配置内容（那需要浏览本机文件、选 profile）。
 *
 * 页内明说这一点，免得用户以为功能坏了。
 */
export function AgentPage() {
  const [clients, setClients] = useState<AgentClient[] | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState('')
  const [tabs, setTabs] = useState<Record<string, string>>({})

  const load = async () => {
    try {
      const data = await fetchAgents()
      setClients(data.clients || [])
      setError('')
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure))
    }
  }

  usePolling(load, 120_000)

  const pull = usePullToRefresh({ onRefresh: load })

  const restore = async (client: AgentClient) => {
    if (!client.id) return
    setBusy(client.id)
    try {
      const preview = await previewAgentRestore(client.id)
      const confirmed = await confirmDialog(
        <div className='flex flex-col gap-3'>
          <p>{t('恢复作用在网关所在机器；密钥已隐藏，不会覆盖应用后的外部修改。')}</p>
          {preview.warnings.map((warning, index) => <p key={index}>{warning}</p>)}
          {preview.files.map(file => <div key={file.path}><code className='break-all'>{file.path}</code>
            <pre className='max-h-60 overflow-auto whitespace-pre text-[11px]'>{file.diff.filter(line => line.kind !== 'same').map(line => `${line.kind === 'added' ? '+' : '−'} ${line.text}`).join('\n')}</pre>
          </div>)}
        </div>,
        { title: t('撤销上次应用'), confirmText: t('确认恢复') },
      )
      if (!confirmed) return
      await restoreAgentClient(client.id, preview.token)
      store.toast(t('配置已恢复并校验'))
      await load()
    } catch (failure) {
      store.toast(failure instanceof Error ? failure.message : String(failure), 'err')
    } finally {
      setBusy('')
    }
  }

  return (
    <div className='m-page' ref={pull.bindRef}>
      <PullIndicator pull={pull.pull} refreshing={pull.refreshing} />
      <PageIntro subtitle={t('改写网关所在机器上的客户端配置')}
        actions={<IconButton name='refresh' label={t('刷新')} onClick={() => void load()} />}
      />

      <div className='m-section' style={{ marginTop: 0 }}>
        <Card
          className='m-card m-card-pad'
          style={{ background: 'var(--m-warn-soft)', borderColor: 'color-mix(in srgb, var(--m-warn) 24%, transparent)' }}
        >
          <div style={{ fontSize: 12.3, lineHeight: 1.6, color: 'var(--m-text-2)' }}>
            {t('手机端可查看模型配置、撤销上次应用，并管理提示词、Skill 与 MCP。')}
            <b style={{ color: 'var(--m-text)' }}>{t('所有操作作用在网关所在机器。')}</b>
          </div>
        </Card>
      </div>

      {error ? <ErrorNote text={error} /> : null}
      {!clients ? <SkeletonList rows={3} height={96} /> : null}

      {clients && !clients.length ? (
        <div className='m-section'>
          <Card className='m-card'>
            <Empty text={t('没有检测到可接管的客户端')} />
          </Card>
        </div>
      ) : null}

      {(clients || []).map(client => (
        <div key={client.id} className='m-section' style={{ marginTop: 10 }}>
          <Card className='m-card m-card-pad'>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
              <GlyphLogo name='agent' />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 14, fontWeight: 600 }}>{client.name || client.id}</div>
                <div
                  className='m-mono'
                  style={{
                    fontSize: 11,
                    color: 'var(--m-text-3)',
                    marginTop: 3,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {client.resolvedPath || client.configPath || '—'}
                </div>
              </div>
              <Badge tone={client.viaGateway ? 'ok' : 'mute'}>{client.viaGateway ? t('接入本网关') : t('未接入本网关')}</Badge>
            </div>
            <SegmentedControl className='mt-3' value={tabs[client.id || ''] || 'config'} onValueChange={value => setTabs(previous => ({ ...previous, [client.id || '']: value }))}
              aria-label={t('客户端管理分类')} options={[{ value: 'config', label: t('模型配置') }, { value: 'prompt', label: t('提示词') }, { value: 'skills', label: 'Skill' }, { value: 'mcp', label: 'MCP' }]} />
            {tabs[client.id || ''] && tabs[client.id || ''] !== 'config' ? <AgentResourcesPanel clientId={client.id || ''} kind={tabs[client.id || ''] as ResourceKind}
              capabilities={client.resources} request={resourceRequest} t={t} notify={(text, bad) => store.toast(text, bad ? 'err' : 'ok')} /> : null}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                marginTop: 11,
                paddingTop: 10,
                borderTop: '1px solid var(--m-line)',
                fontSize: 11.5,
                color: 'var(--m-text-3)',
              }}
            >
              <span>{client.detectionState === 'missing' ? t('未检测到') : client.detectionState === 'uninitialized' ? t('尚未初始化') : client.detectionState === 'read_error' ? t('读取失败') : client.detectionState === 'unsupported' ? t('仅可读取') : t('客户端原生配置')}</span>
              {client.readError ? <span style={{ color: 'var(--m-danger)' }}>{client.readError}</span> : null}
              {client.applied?.canUndo ? (
                <Button size='touch-sm' variant='outline'
                  type='button'
                  className='m-btn m-sm m-ghost'
                  style={{ marginLeft: 'auto', height: 28, fontSize: 11.5, padding: '0 10px' }}
                  disabled={busy === client.id}
                  onClick={() => void restore(client)}
                >
                  <Icon name='refresh' style={{ width: 12, height: 12 }} />
                  {busy === client.id ? t('还原中…') : t('撤销上次应用')}
                </Button>
              ) : null}
            </div>
          </Card>
        </div>
      ))}

      <div className='m-section' style={{ paddingBottom: 8 }}>
        <Card className='m-card m-card-pad' style={{ fontSize: 11.5, color: 'var(--m-text-3)', lineHeight: 1.7 }}>
          {t('接入状态来自客户端配置文件。恢复前显示差异；文件被其他程序修改时拒绝覆盖。')}
        </Card>
      </div>
    </div>
  )
}
