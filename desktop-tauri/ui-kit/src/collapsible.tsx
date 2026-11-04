import * as React from 'react'
import { Collapsible as BaseCollapsible } from '@base-ui/react/collapsible'
import { Button } from './button'
import { cn } from './lib/cn'

function Collapsible(props: BaseCollapsible.Root.Props) {
  return <BaseCollapsible.Root data-slot='collapsible' {...props} />
}

function CollapsibleTrigger({ className, render, ...props }: BaseCollapsible.Trigger.Props) {
  return <BaseCollapsible.Trigger data-slot='collapsible-trigger' render={render ?? <Button variant='ghost' />}
    className={className} {...props} />
}

function CollapsibleContent({ className, ...props }: BaseCollapsible.Panel.Props) {
  return <BaseCollapsible.Panel data-slot='collapsible-content' className={cn('min-w-0', typeof className === 'string' && className)} {...props} />
}

export { Collapsible, CollapsibleTrigger, CollapsibleContent }
