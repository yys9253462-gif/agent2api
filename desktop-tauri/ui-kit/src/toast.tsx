import * as React from 'react'
import { Toast as BaseToast } from '@base-ui/react/toast'
import { cn } from './lib/cn'

const createToastManager = BaseToast.createToastManager
const toastManager = createToastManager()
const ToastProvider = BaseToast.Provider
const ToastClose = BaseToast.Close

function ToastTitle({ className, ...props }: BaseToast.Title.Props) {
  return <BaseToast.Title data-slot='toast-title' className={state => cn(
    'm-0 text-[12.5px] font-semibold leading-relaxed [overflow-wrap:anywhere]',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function ToastDescription({ className, ...props }: BaseToast.Description.Props) {
  return <BaseToast.Description data-slot='toast-description' className={state => cn(
    'm-0 text-[12px] leading-relaxed text-subtle [overflow-wrap:anywhere]',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function Toast({ className, ...props }: BaseToast.Root.Props) {
  return <BaseToast.Root data-slot='toast' className={cn(
    'pointer-events-auto max-w-full rounded-lg border border-border bg-raised px-4 py-2.5 text-[12.5px] text-foreground shadow-3 outline-none',
    'data-[type=success]:border-success-bd data-[type=success]:text-success',
    'data-[type=error]:border-destructive-bd data-[type=error]:text-destructive',
    'data-[type=warning]:border-warning-bd data-[type=warning]:text-warning',
    'data-limited:hidden data-ending-style:opacity-0 transition-opacity duration-150',
    typeof className === 'string' && className,
  )} {...props} />
}

function ToastList() {
  const { toasts } = BaseToast.useToastManager()
  return toasts.map(toast => <Toast key={toast.id} toast={toast}>
    <ToastTitle>{toast.title}</ToastTitle>
    {toast.description && <ToastDescription className='mt-1'>{toast.description}</ToastDescription>}
  </Toast>)
}

function Toaster({ manager = toastManager, position = 'top-center', className }: {
  manager?: ReturnType<typeof createToastManager>
  position?: 'top-center' | 'bottom-center'
  className?: string
}) {
  return <ToastProvider toastManager={manager} timeout={3500} limit={1}>
    <BaseToast.Portal>
      <BaseToast.Viewport data-slot='toast-viewport' className={cn(
        'pointer-events-none fixed left-1/2 z-[100] flex w-max max-w-[84vw] -translate-x-1/2 flex-col items-center gap-2 outline-none',
        position === 'top-center' ? 'top-4' : 'bottom-[calc(78px+env(safe-area-inset-bottom,0px))]',
        className,
      )}>
        <ToastList />
      </BaseToast.Viewport>
    </BaseToast.Portal>
  </ToastProvider>
}

export { Toast, ToastProvider, ToastTitle, ToastDescription, ToastClose, Toaster, toastManager, createToastManager }
