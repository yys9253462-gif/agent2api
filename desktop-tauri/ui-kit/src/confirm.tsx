import * as React from 'react'
import { createRoot } from 'react-dom/client'
import { AlertDialog, AlertDialogBody, AlertDialogContent, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from './alert-dialog'
import { Button } from './button'
import { t } from './i18n'

function confirmDialog(message: React.ReactNode, options: { title?: string; confirmText?: string; destructive?: boolean } = {}): Promise<boolean> {
  return new Promise(resolve => {
    const container = document.createElement('div')
    document.body.append(container)
    const root = createRoot(container)
    const cancelRef = React.createRef<HTMLButtonElement>()
    let settled = false
    const finish = (value: boolean) => {
      if (settled) return
      settled = true
      queueMicrotask(() => { root.unmount(); container.remove() })
      resolve(value)
    }
    root.render(<AlertDialog open onOpenChange={open => { if (!open) finish(false) }}>
      <AlertDialogContent initialFocus={cancelRef}>
        <AlertDialogHeader><AlertDialogTitle>{options.title || t('请确认')}</AlertDialogTitle></AlertDialogHeader>
        <AlertDialogBody><div className='whitespace-pre-wrap text-[13px] leading-relaxed'>{message}</div></AlertDialogBody>
        <AlertDialogFooter>
          <Button ref={cancelRef} variant='outline' onClick={() => finish(false)}>{t('取消')}</Button>
          <Button variant={options.destructive === false ? 'default' : 'destructive'} onClick={() => finish(true)}>{options.confirmText || t('确定')}</Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>)
  })
}

export { confirmDialog }
