import * as React from 'react'
import { Card } from './card'
import { cn } from './lib/cn'

function MetricCard({ className, ...props }: React.ComponentProps<typeof Card>) {
  return <Card data-slot='metric-card' className={cn('rounded-md bg-surface-inset px-3.5 py-3', className)} {...props} />
}

function MetricLabel({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='metric-label' className={cn('mb-1 text-[11.5px] text-muted-foreground', className)} {...props} />
}

function MetricValue({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='metric-value' className={cn('break-all text-[13px] font-semibold leading-normal tabular-nums', className)} {...props} />
}

function MetricGrid({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='metric-grid' className={cn('grid grid-cols-[repeat(auto-fit,minmax(min(178px,100%),1fr))] gap-2.5', className)} {...props} />
}

export { MetricCard, MetricLabel, MetricValue, MetricGrid }
