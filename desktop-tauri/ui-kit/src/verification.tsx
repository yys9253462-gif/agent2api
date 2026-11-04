import * as React from 'react'
import { Button } from './button'
import { StatusDot } from './status-dot'
import { Spinner } from './spinner'
import { cn } from './lib/cn'

function Verification({ state = 'idle', children, className, ...props }: React.ComponentProps<typeof Button> & {
  state?: 'idle' | 'verifying' | 'verified' | 'error'
}) {
  return <Button variant='outline' data-slot='verification' className={cn('h-auto min-h-10 gap-3 px-3.5 py-2.5 text-[13px] disabled:opacity-100', state === 'verified' && 'text-success', state === 'error' && 'border-destructive-bd text-destructive', className)} {...props}>
    {state === 'verifying' ? <Spinner /> : <StatusDot variant={state === 'verified' ? 'success' : state === 'error' ? 'destructive' : 'muted'} />}
    <span className='min-w-0'>{children}</span>
  </Button>
}

export { Verification }
