import * as React from 'react'
import { cn } from './lib/cn'

type StatusDotProps = React.ComponentProps<'span'> & { variant?: 'success' | 'warning' | 'destructive' | 'muted'; pulse?: boolean }

function StatusDot({ className, variant = 'success', pulse, ...props }: StatusDotProps) {
  return <span data-slot='status-dot' aria-hidden='true' className={cn('inline-block size-[7px] shrink-0 rounded-full',
    variant === 'success' && 'bg-success', variant === 'warning' && 'bg-warning',
    variant === 'destructive' && 'bg-destructive', variant === 'muted' && 'bg-muted-foreground',
    pulse && 'animate-pulse motion-reduce:animate-none', className,
  )} {...props} />
}

export { StatusDot, type StatusDotProps }
