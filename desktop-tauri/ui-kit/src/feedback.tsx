import * as React from 'react'
import { cn } from './lib/cn'

function Alert({ className, variant = 'default', ...props }: React.ComponentProps<'div'> & {
  variant?: 'default' | 'destructive' | 'warning' | 'success'
}) {
  return <div data-slot='alert' role='alert' className={cn(
    'rounded-md border px-3 py-2.5 text-[12.5px] leading-relaxed',
    variant === 'default' && 'border-border bg-surface-2 text-subtle',
    variant === 'destructive' && 'border-destructive-bd bg-destructive-soft text-destructive',
    variant === 'warning' && 'border-warning-bd bg-warning-soft text-warning',
    variant === 'success' && 'border-success-bd bg-success-soft text-success',
    className,
  )} {...props} />
}

function AlertTitle({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='alert-title' className={cn('font-semibold', className)} {...props} />
}

function AlertDescription({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='alert-description' className={cn('text-[12px] leading-relaxed', className)} {...props} />
}

function Empty({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='empty' className={cn('flex flex-col items-center justify-center gap-2 px-4 py-8 text-center text-[12.5px] leading-relaxed text-muted-foreground', className)} {...props} />
}

function EmptyTitle({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='empty-title' className={cn('text-[13px] font-semibold text-foreground', className)} {...props} />
}

function EmptyDescription({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='empty-description' className={cn('text-[12px] leading-relaxed text-muted-foreground', className)} {...props} />
}

export { Alert, AlertTitle, AlertDescription, Empty, EmptyTitle, EmptyDescription }
