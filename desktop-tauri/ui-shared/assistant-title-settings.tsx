import { useEffect, useState } from 'react'
import { Alert, Button, Dialog, DialogContent, DialogHeader, DialogTitle, DialogBody, DialogFooter, Label, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Textarea } from '@ui'
import { DEFAULT_TITLE_PROMPT } from './assistant-history'
import type { Translate } from './assistant-read-types'

export function AssistantTitleSettings({ open, onOpenChange, model, prompt, models, disabled, onSave, t }: {
  open: boolean
  onOpenChange: (open: boolean) => void
  model: string | null
  prompt: string
  models: string[]
  disabled?: boolean
  onSave: (model: string | null, prompt: string) => void | Promise<void>
  t: Translate
}) {
  const [selected, setSelected] = useState('')
  const [draft, setDraft] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  useEffect(() => {
    if (open) { setSelected(model || ''); setDraft(prompt || DEFAULT_TITLE_PROMPT); setError('') }
  }, [open, model, prompt])
  return <Dialog open={open} onOpenChange={value => { if (!saving) onOpenChange(value) }}>
    <DialogContent>
      <DialogHeader><DialogTitle>{t('标题总结设置')}</DialogTitle></DialogHeader>
      <DialogBody>
        <Label>{t('标题总结模型')}</Label>
        <Select value={selected || '__follow__'} disabled={disabled || saving} onValueChange={value => setSelected(value === '__follow__' ? '' : String(value))}>
          <SelectTrigger aria-label={t('标题总结模型')}><SelectValue>{selected || t('跟随对话模型')}</SelectValue></SelectTrigger>
          <SelectContent>
            <SelectItem value='__follow__'>{t('跟随对话模型')}</SelectItem>
            {selected && !models.includes(selected) ? <SelectItem value={selected} disabled>{selected} · {t('不可用')}</SelectItem> : null}
            {models.map(name => <SelectItem key={name} value={name}>{name}</SelectItem>)}
          </SelectContent>
        </Select>
        <Label>{t('标题总结提示词')}</Label>
        <Textarea rows={7} value={draft} maxLength={8000} disabled={saving} onChange={event => setDraft(event.currentTarget.value)} />
        <p className='text-xs text-muted-foreground'>{t('首轮回答完成后自动总结；这会调用所选模型并消耗额度。')}</p>
        {error ? <Alert variant='destructive'>{error}</Alert> : null}
      </DialogBody>
      <DialogFooter>
        <Button variant='outline' disabled={saving} onClick={() => setDraft(DEFAULT_TITLE_PROMPT)}>{t('恢复默认')}</Button>
        <Button variant='outline' disabled={saving} onClick={() => onOpenChange(false)}>{t('取消')}</Button>
        <Button disabled={disabled || saving} onClick={async () => {
          setSaving(true); setError('')
          try { await onSave(selected || null, draft.trim() === DEFAULT_TITLE_PROMPT ? '' : draft); onOpenChange(false) }
          catch (failure) { setError(t('保存失败：{message}', { message: failure instanceof Error ? failure.message : String(failure) })) }
          finally { setSaving(false) }
        }}>{t('保存')}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
}
