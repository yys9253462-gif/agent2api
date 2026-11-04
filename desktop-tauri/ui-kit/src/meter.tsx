import * as React from 'react'
import { Meter as BaseMeter } from '@base-ui/react/meter'
import { cn } from './lib/cn'

type MeterProps = Omit<BaseMeter.Root.Props, 'value' | 'className'> & {
  value: number | null
  variant?: 'default' | 'success' | 'warning' | 'destructive' | 'muted'
  className?: string
  trackClassName?: string
  indicatorClassName?: string
}

function Meter({ value, variant = 'default', className, trackClassName, indicatorClassName, ...props }: MeterProps) {
  const track = cn('block h-1 w-full overflow-hidden rounded-pill bg-hairline', trackClassName)
  if (value === null) return <span data-slot='meter' aria-hidden='true' className={cn('block w-full', className)}><span data-slot='meter-track' className={track} /></span>
  return <BaseMeter.Root render={<span />} value={value} data-slot='meter' className={cn('block w-full', className)} {...props}>
    <BaseMeter.Track render={<span />} data-slot='meter-track' className={track}>
      <BaseMeter.Indicator render={<span />} data-slot='meter-indicator' className={cn('block h-full rounded-[inherit] transition-[width] duration-300 motion-reduce:transition-none',
        variant === 'default' && 'bg-primary', variant === 'success' && 'bg-success',
        variant === 'warning' && 'bg-warning', variant === 'destructive' && 'bg-destructive',
        variant === 'muted' && 'bg-muted-foreground', indicatorClassName)} />
    </BaseMeter.Track>
  </BaseMeter.Root>
}

export { Meter, type MeterProps }
