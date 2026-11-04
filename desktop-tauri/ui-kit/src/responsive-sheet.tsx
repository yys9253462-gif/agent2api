import * as React from 'react'
import { cn } from './lib/cn'
import { Sheet, SheetBody, SheetContent, SheetHeader, SheetTitle } from './sheet'

function useMediaQuery(query: string) {
  const media = React.useMemo(() => typeof window === 'undefined' ? null : window.matchMedia(query), [query])
  return React.useSyncExternalStore(
    React.useCallback(listener => {
      media?.addEventListener('change', listener)
      return () => media?.removeEventListener('change', listener)
    }, [media]),
    () => media?.matches ?? false,
    () => false,
  )
}

type ResponsiveSheetProps = {
  open: boolean
  onOpenChange(open: boolean): void
  title: React.ReactNode
  actions?: React.ReactNode
  children: React.ReactNode
  breakpoint?: number
  side?: 'left' | 'right'
  inlineClassName?: string
  inlineAs?: 'div' | 'aside'
  contentClassName?: string
  bodyClassName?: string
}

function ResponsiveSheet({
  open, onOpenChange, title, actions, children, breakpoint = 767, side = 'right',
  inlineClassName, inlineAs: Inline = 'div', contentClassName, bodyClassName,
}: ResponsiveSheetProps) {
  const narrow = useMediaQuery(`(max-width: ${breakpoint}px)`)
  React.useEffect(() => { if (!narrow && open) onOpenChange(false) }, [narrow, open, onOpenChange])
  React.useEffect(() => {
    const close = () => onOpenChange(false)
    window.addEventListener('wb:navigation-close', close)
    return () => window.removeEventListener('wb:navigation-close', close)
  }, [onOpenChange])
  if (!narrow) return <Inline data-slot='responsive-sheet-inline' className={inlineClassName}>{children}</Inline>
  return <Sheet open={open} onOpenChange={onOpenChange}>
    <SheetContent side={side} keepMounted showHandle={false} className={contentClassName}>
      <SheetHeader><SheetTitle>{title}</SheetTitle>{actions}</SheetHeader>
      <SheetBody className={cn('gap-4 [&>*]:w-full [&>*]:min-w-0 [&>*]:max-w-none', bodyClassName)}>{children}</SheetBody>
    </SheetContent>
  </Sheet>
}

export { ResponsiveSheet, useMediaQuery, type ResponsiveSheetProps }
