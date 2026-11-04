import * as React from 'react'
import { Alert, Badge, Button, Card, CardContent, CardHeader, CardTitle, Empty } from '@ui'
import { t } from '../i18n'
import { confirm, errorMessage, getAgentStore, importFromClient, saveProfiles, subscribeAgentStore, toast } from './agent-state'
import { blankProfile, defaultOperation, kindLabel, protocolLabel } from './agent-model'
import { ProfileDialog } from './agent-profile-dialog'
import { ApplyDialog } from './agent-preview'
import type { AgentClient, AgentOperation, AgentProfile } from './agent-types'

export function ProfileEditor({ client }: { client: AgentClient }) {
  const store = React.useSyncExternalStore(subscribeAgentStore,getAgentStore,getAgentStore)
  const [editor,setEditor] = React.useState<{ profile: AgentProfile; creating: boolean; native?: boolean } | null>(null)
  const [applying,setApplying] = React.useState<{ profile: AgentProfile; operation: AgentOperation } | null>(null)
  const disabled = Boolean(store.busy) || store.loading
  const remove = async (profile: AgentProfile) => {
    if (!await confirm(t('删除已保存方案？'),t('只删除 Agent2API 中的方案，不修改客户端当前配置。'),t('删除方案'))) return
    try { await saveProfiles(client,client.profiles.filter(item => item.id !== profile.id)); toast(t('已删除保存方案，客户端配置未改动')) }
    catch (error) { toast(errorMessage(error),'err') }
  }
  return <>
    <Card className='agent-card'>
      <CardHeader className='agent-card-head'><CardTitle>{t('客户端实际配置')}</CardTitle><span className='mr-auto' />
        <Button variant='ghost' size='sm' disabled={disabled || Boolean(client.readError) || !client.live.length} onClick={() => void importFromClient(client)}>{t('从客户端导入')}</Button></CardHeader>
      <CardContent className='agent-profiles'>
        {client.readError && <Alert variant='destructive'>{client.readError}</Alert>}
        {!client.readError && !client.live.length && <Empty>{t('未读取到可识别的客户端配置')}</Empty>}
        {client.live.map(profile => <ProfileRow key={profile.id} profile={profile} live client={client}>
          <Button variant='ghost' size='xs' disabled={disabled || !profile.editable} onClick={() => setEditor({ profile, creating:false, native:true })}>{t('编辑')}</Button>
          <Button variant='ghost' size='xs' disabled={disabled || !profile.deletable} onClick={() => setApplying({ profile, operation:'native-delete' })}>{t('删除')}</Button>
        </ProfileRow>)}
        {client.live.length > 0 && <p className='text-xs leading-relaxed text-subtle'>{t('此处来自磁盘文件，不代表正在运行的旧会话已经重载。')}</p>}
      </CardContent>
    </Card>
    <Card className='agent-card'>
      <CardHeader className='agent-card-head'><CardTitle>{t('已保存方案')}</CardTitle><span className='mr-auto' />
        <Button variant='outline' size='sm' disabled={disabled || !client.writable || Boolean(client.storeError)} onClick={() => setEditor({profile:blankProfile(client,store.gateway),creating:true})}>＋ {t('新建配置')}</Button></CardHeader>
      <CardContent className='agent-profiles'>
        {client.storeError && <Alert variant='destructive'>{client.storeError}</Alert>}
        {!client.profiles.length && <Empty>{t('新建方案或导入当前配置；保存后再决定是否应用。')}</Empty>}
        {client.profiles.map(profile => <ProfileRow key={profile.id} profile={profile} client={client}>
          <Button variant='ghost' size='xs' disabled={disabled || !client.writable} onClick={() => setEditor({profile,creating:false})}>{t('编辑')}</Button>
          <Button variant='ghost' size='xs' disabled={disabled} onClick={() => void remove(profile)}>{t('删除')}</Button>
          <Button variant='outline' size='xs' disabled={disabled || !client.writable || Boolean(profile.validationError)} onClick={() => setApplying({profile,operation:defaultOperation(client)})}>{client.configMode === 'single' ? t('使用此配置') : t('应用')}</Button>
          {client.configMode !== 'single' && client.supportsDefault && <Button variant='ghost' size='xs' disabled={disabled || !client.writable || Boolean(profile.validationError)} onClick={() => setApplying({profile,operation:'default'})}>{t('应用并设为默认')}</Button>}
        </ProfileRow>)}
      </CardContent>
    </Card>
    {editor && <ProfileDialog key={`${client.id}-${editor.profile.id}`} client={client} profile={editor.profile} creating={editor.creating} native={editor.native} onClose={() => setEditor(null)} />}
    {applying && <ApplyDialog client={client} profile={applying.profile} operation={applying.operation} onClose={() => setApplying(null)} />}
  </>
}

function ProfileRow({ client,profile,live = false,children }: { client: AgentClient; profile: AgentProfile; live?: boolean; children?: React.ReactNode }) {
  const status = live ? client.supportsToggle ? profile.enabled ? t('已启用') : t('已停用') : profile.enabled ? t('当前默认') : t('已配置') : profile.inFile ? t('与客户端一致') : t('已保存，未应用')
  return <div className='agent-profile agent-profile-v2'>
    <div className='agent-profile-main'>
      <div className='agent-profile-name'><span>{profile.name || profile.id}</span><Badge variant={live && profile.enabled || profile.inFile ? 'success' : 'outline'} className='agent-tag'>{status}</Badge>
        <Badge variant='outline' className='agent-tag'>{kindLabel(profile.kind)}</Badge></div>
      <div className='agent-profile-sub'><code className='break-all'>{profile.baseUrl || t('客户端默认地址')}</code><span>{protocolLabel(profile.protocol)}</span></div>
      {profile.models.length > 0 ? <div className='agent-profile-models'>{profile.models.map(model => <span key={model.id} className='text-xs font-mono'>{model.id}</span>)}</div>
        : profile.model ? <code className='text-xs'>{profile.model}</code> : null}
      {profile.note && <p className='agent-profile-note'>{profile.note}</p>}
      {profile.validationError && <p className='text-xs text-destructive'>{t('需要重新编辑：')} {profile.validationError}</p>}
    </div>
    {children && <div className='agent-profile-act flex-wrap'>{children}</div>}
  </div>
}
