import * as React from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn } from './lib/cn'

type CardProps = useRender.ComponentProps<'div'> & { size?: 'default' | 'sm' }

function Card({ className, size = 'default', render, ...props }: CardProps) {
  return useRender({
    defaultTagName: 'div',
    render,
    props: {
      ...props,
      'data-slot': 'card',
      'data-size': size,
      className: cn('group/card min-w-0 overflow-hidden rounded-lg border border-border bg-surface text-foreground shadow-1', className),
    },
  })
}

function CardHeader({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='card-header' className={cn('flex flex-wrap items-center gap-2.5 border-b border-border px-4 py-3 group-data-[size=sm]/card:px-3 group-data-[size=sm]/card:py-2', className)} {...props} />
}

function CardTitle({ className, ...props }: React.ComponentProps<'h3'>) {
  return <h3 data-slot='card-title' className={cn('m-0 text-[14px] font-semibold leading-snug', className)} {...props} />
}

function CardDescription({ className, ...props }: React.ComponentProps<'p'>) {
  return <p data-slot='card-description' className={cn('m-0 text-[12px] leading-relaxed text-subtle', className)} {...props} />
}

function CardAction({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='card-action' className={cn('ml-auto flex shrink-0 flex-wrap items-center gap-2', className)} {...props} />
}

function CardContent({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='card-content' className={cn('min-w-0 p-4 group-data-[size=sm]/card:p-3', className)} {...props} />
}

function CardFooter({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='card-footer' className={cn('flex flex-wrap items-center gap-2 border-t border-border px-4 py-2.5 text-[11.5px] leading-relaxed text-muted-foreground', className)} {...props} />
}

export { Card, CardHeader, CardTitle, CardDescription, CardAction, CardContent, CardFooter, type CardProps }
