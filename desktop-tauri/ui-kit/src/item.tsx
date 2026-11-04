import * as React from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn } from './lib/cn'

function Item({ className, render, variant = 'default', size = 'default', ...props }: useRender.ComponentProps<'div'> & {
  variant?: 'default' | 'outline' | 'muted'
  size?: 'default' | 'sm'
}) {
  return useRender({
    defaultTagName: 'div',
    render,
    props: {
      ...props,
      'data-slot': 'item',
      'data-size': size,
      className: cn(
        'flex w-full min-w-0 items-center gap-2.5 rounded-md text-left text-foreground transition-colors',
        size === 'sm' ? 'px-3 py-2 text-[12.5px]' : 'px-3.5 py-3 text-[13px]',
        variant === 'outline' && 'border border-border',
        variant === 'muted' && 'bg-surface-2',
        'has-[[data-slot=item-content]]:justify-start',
        className,
      ),
    },
  })
}

function ItemGroup({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='item-group' role='list' className={cn('flex flex-col', className)} {...props} />
}

function ItemContent({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='item-content' className={cn('min-w-0 flex-1', className)} {...props} />
}

function ItemTitle({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='item-title' className={cn('text-[13px] font-medium', className)} {...props} />
}

function ItemDescription({ className, ...props }: React.ComponentProps<'p'>) {
  return <p data-slot='item-description' className={cn('m-0 mt-0.5 text-[12px] leading-relaxed text-muted-foreground', className)} {...props} />
}

function ItemMedia({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='item-media' className={cn('flex shrink-0 items-center justify-center', className)} {...props} />
}

function ItemActions({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='item-actions' className={cn('ml-auto flex shrink-0 items-center gap-2', className)} {...props} />
}

export { Item, ItemGroup, ItemContent, ItemTitle, ItemDescription, ItemMedia, ItemActions }
