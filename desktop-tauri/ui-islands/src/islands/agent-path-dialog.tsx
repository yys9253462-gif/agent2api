import * as React from 'react'
import { Alert, Button, Dialog, DialogBody, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@ui'
import { t } from '../i18n'
import { TextField } from './agent-fields'
import { errorMessage, savePath } from './agent-state'
import type { AgentClient } from './agent-types'

export function PathDialog({client,onClose}: {client:AgentClient;onClose:()=>void}) {
  const [path,setPath] = React.useState(client.pathOverride || client.configPath)
  const [saving,setSaving] = React.useState(false)
  const [error,setError] = React.useState('')
  const save = async (path:string) => {
    setSaving(true);setError('')
    try { await savePath(client,path);onClose() }
    catch(error) {setError(errorMessage(error));setSaving(false)}
  }
  return <Dialog open onOpenChange={(open,details)=>{if(!open){if(saving)details.cancel();else onClose()}}}>
    <DialogContent><DialogHeader><DialogTitle>{t('指定配置路径')} · {client.name}</DialogTitle></DialogHeader>
      <DialogBody><Alert>{t('路径属于网关所在机器，不是浏览器所在设备。只改变检测目标，不修改客户端文件。')}</Alert>
        <TextField label={t('完整配置文件路径')} fieldKey='' value={path} onChange={setPath} disabled={saving} />
        <p className='text-xs text-subtle'>{t('客户端必须实际使用同一个配置文件；本页不改变客户端启动参数。')}</p>
        {error&&<Alert variant='destructive'>{error}</Alert>}
      </DialogBody>
      <DialogFooter><Button variant='ghost' disabled={saving} onClick={()=>void save('')}>{t('恢复自动检测')}</Button><span className='mr-auto'/>
        <Button variant='outline' disabled={saving} onClick={onClose}>{t('取消')}</Button><Button disabled={saving} onClick={()=>void save(path)}>{t('保存路径')}</Button></DialogFooter>
    </DialogContent>
  </Dialog>
}
