import * as React from 'react'
import { Button } from './button'
import { cn } from './lib/cn'

function AppFrame({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='app-frame' className={cn('min-h-0 bg-background text-foreground', className)} {...props} />
}

function Sidebar({ className, ...props }: React.ComponentProps<'aside'>) {
  return <aside data-slot='sidebar' className={cn('flex min-h-0 flex-col gap-3 border-r border-sidebar-border bg-sidebar p-3 text-sidebar-foreground', className)} {...props} />
}

function SidebarHeader({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='sidebar-header' className={cn('shrink-0', className)} {...props} />
}

function SidebarContent({ className, ...props }: React.ComponentProps<'nav'>) {
  return <nav data-slot='sidebar-content' className={cn('flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto', className)} {...props} />
}

function SidebarFooter({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='sidebar-footer' className={cn('flex shrink-0 flex-col gap-2 border-t border-sidebar-border pt-2.5', className)} {...props} />
}

function SidebarGroupLabel({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='sidebar-group-label' className={cn('px-2 pb-1 pt-2 text-[10.5px] font-semibold uppercase tracking-[.09em] text-sidebar-muted', className)} {...props} />
}

function PageHeader({ className, ...props }: React.ComponentProps<'header'>) {
  return <header data-slot='page-header' className={cn('flex shrink-0 items-center justify-between gap-3', className)} {...props} />
}

function PageTitle({ className, ...props }: React.ComponentProps<'h1'>) {
  return <h1 data-slot='page-title' className={cn('m-0 text-[20px] font-semibold tracking-tight', className)} {...props} />
}

function PageDescription({ className, ...props }: React.ComponentProps<'p'>) {
  return <p data-slot='page-description' className={cn('m-0 text-[12.5px] text-muted-foreground', className)} {...props} />
}

function BottomNavigation({ className, ...props }: React.ComponentProps<'nav'>) {
  return <nav data-slot='bottom-navigation' className={cn('flex shrink-0 items-stretch border-t border-border bg-surface px-1.5 pt-1.5 pb-[env(safe-area-inset-bottom,0px)]', className)} {...props} />
}

function BottomNavigationItem({ className, active, ...props }: React.ComponentProps<typeof Button> & { active?: boolean }) {
  return <Button data-slot='bottom-navigation-item' aria-current={active ? 'page' : undefined}
    variant='ghost' className={cn('h-auto min-w-0 flex-1 flex-col gap-0.5 rounded-sm px-1 py-1 text-[10.5px]', active && 'bg-primary-soft font-semibold text-primary-fg', className)} {...props} />
}

function WindowTitleBar({ className, ...props }: React.ComponentProps<'header'>) {
  return <header data-slot='window-titlebar' data-tauri-drag-region='' className={cn('flex h-8 shrink-0 items-center gap-2.5 overflow-hidden border-b border-sidebar-border bg-sidebar pl-3 text-sidebar-fg select-none', className)} {...props} />
}

function WindowControl({ className, action, ...props }: React.ComponentProps<typeof Button> & {
  action: 'minimize' | 'maximize' | 'close'
}) {
  return <Button data-slot='window-control' variant='ghost' className={cn('h-8 w-11 rounded-none p-0 shadow-none active:translate-y-0', action === 'close' && 'hover:bg-destructive hover:text-white', className)} {...props} />
}

export { AppFrame, Sidebar, SidebarHeader, SidebarContent, SidebarFooter, SidebarGroupLabel, PageHeader, PageTitle, PageDescription, BottomNavigation, BottomNavigationItem, WindowTitleBar, WindowControl }
