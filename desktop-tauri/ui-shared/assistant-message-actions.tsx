import { useState } from 'react'
import { Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogBody, DialogFooter, Textarea, confirmDialog } from '@ui'
import type { Translate } from './assistant-read-types'

type Props = {
  text: string
  disabled?: boolean
  t: Translate
  onEdit: (text: string) => void | Promise<void>
  onDelete: () => void | Promise<void>
  onFork?: () => void | Promise<void>
}

function ActionIcon({ action }: { action: 'edit' | 'delete' | 'fork' }) {
  return <svg viewBox='0 0 24 24' width='15' height='15' fill='none' stroke='currentColor' strokeWidth='1.8' strokeLinecap='round' strokeLinejoin='round' aria-hidden='true'>
    {action === 'edit' ? <><path d='m15.6 4.6 3.8 3.8L9 18.8l-4.6.8.8-4.6z' /><path d='m13.8 6.4 3.8 3.8' /></>
      : action === 'delete' ? <><path d='M5 7h14M9.5 7V5.2h5V7M7 7l.9 12.4h8.2L17 7M10.5 10.5v5.5M13.5 10.5v5.5' /></>
        : <><circle cx='6' cy='5' r='2' /><circle cx='6' cy='19' r='2' /><circle cx='18' cy='5' r='2' /><path d='M6 7v10M18 7v1a5 5 0 0 1-5 5H6' /></>}
  </svg>
}

export function AssistantMessageActions({ text, disabled, t, onEdit, onDelete, onFork }: Props) {
  const [open, setOpen] = useState(false)
  const [draft, setDraft] = useState('')
  const [saving, setSaving] = useState(false)
  return <>
    <div className='flex items-center gap-1 pt-1 text-muted-foreground' aria-label={t('消息操作')}>
      <Button size='icon-sm' variant='ghost' disabled={disabled} title={t('编辑')} aria-label={t('编辑')} onClick={() => { setDraft(text); setOpen(true) }}><ActionIcon action='edit' /></Button>
      <Button size='icon-sm' variant='ghost' disabled={disabled} title={t('删除')} aria-label={t('删除')} onClick={async () => {
        if (await confirmDialog(t('删除这条消息？模型消息关联的工具结果也会一并删除。'), { title: t('删除消息') })) await onDelete()
      }}><ActionIcon action='delete' /></Button>
      {onFork ? <Button size='icon-sm' variant='ghost' disabled={disabled} title={t('分叉')} aria-label={t('分叉')} onClick={() => void onFork()}><ActionIcon action='fork' /></Button> : null}
    </div>
    <Dialog open={open} onOpenChange={value => { if (!saving) setOpen(value) }}>
      <DialogContent>
        <DialogHeader><DialogTitle>{t('编辑消息')}</DialogTitle></DialogHeader>
        <DialogBody><Textarea rows={10} value={draft} onChange={event => setDraft(event.currentTarget.value)} disabled={saving} /></DialogBody>
        <DialogFooter>
          <Button variant='outline' disabled={saving} onClick={() => setOpen(false)}>{t('取消')}</Button>
          <Button disabled={saving || disabled} onClick={async () => {
            setSaving(true)
            try { await onEdit(draft); setOpen(false) } finally { setSaving(false) }
          }}>{t('保存')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </>
}
