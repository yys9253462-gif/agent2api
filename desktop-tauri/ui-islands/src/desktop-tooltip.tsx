import * as React from 'react'
import { flushSync } from 'react-dom'
import { createRoot } from 'react-dom/client'
import { Tooltip, TooltipContent, TooltipProvider } from '@ui'

const id = 'wb-tip-bubble'
const registry = new WeakSet<Element>()
let anchor: Element | null = null
let pinned = false
let showTimer = 0
let hideTimer = 0
let revision = 0
const listeners = new Set<() => void>()
const notify = () => { revision += 1; for (const listener of listeners) listener() }
const data = (el: Element) => ({ text: el.getAttribute('data-tip')?.trim() || '', variant: el.getAttribute('data-tip-variant') })
let sizeWatcher: ResizeObserver | null = null

function visible(el: Element) {
  const rect = el.getBoundingClientRect()
  return el.isConnected && rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < window.innerHeight
}

function close() {
  window.clearTimeout(showTimer)
  window.clearTimeout(hideTimer)
  sizeWatcher?.disconnect()
  anchor?.classList.remove('active')
  anchor?.removeAttribute('aria-describedby')
  anchor = null
  pinned = false
  notify()
}

function open(el: Element) {
  if (!data(el).text || !visible(el)) return
  window.clearTimeout(showTimer)
  window.clearTimeout(hideTimer)
  if (anchor && anchor !== el) close()
  anchor = el
  if (!sizeWatcher && typeof ResizeObserver === 'function') {
    sizeWatcher = new ResizeObserver(() => { if (anchor && !visible(anchor)) close() })
  }
  sizeWatcher?.observe(el)
  anchor.classList.add('active')
  anchor.setAttribute('aria-describedby', id)
  notify()
}

function Bubble() {
  React.useSyncExternalStore(listener => { listeners.add(listener); return () => listeners.delete(listener) }, () => revision)
  const current = anchor ? data(anchor) : null
  return <TooltipProvider><Tooltip open={Boolean(anchor)} disableHoverablePopup onOpenChange={next => { if (!next && !pinned) close() }}>
    <TooltipContent id={id} anchor={anchor} side='bottom' variant={current?.variant === 'plain' ? 'plain' : 'default'} className='pointer-events-none'>
      {current?.text.split('\n').map((line, index) => {
        const match = /^\[(#[0-9a-fA-F]{3}|#[0-9a-fA-F]{6})\]/.exec(line)
        return <span key={index} className='flex items-start gap-1.5'>
          {match && <span className='mt-1 size-1.5 shrink-0 rounded-full' style={{ background: match[1] }} />}
          <span>{match ? line.slice(match[0].length) : line}</span>
        </span>
      })}
    </TooltipContent>
  </Tooltip></TooltipProvider>
}

function enhance(el: Element) {
  if (registry.has(el) || !data(el).text) return
  registry.add(el)
  if (el.classList.contains('tip-q') && !el.textContent?.trim()) el.textContent = '?'
  if (!el.matches('a[href], button, input, select, textarea, [tabindex]')) el.setAttribute('tabindex', '0')
}

function boot() {
  const container = document.createElement('div')
  document.body.append(container)
  flushSync(() => createRoot(container).render(<Bubble />))
  const scan = (node: Element) => {
    if (node.matches('[data-tip]')) enhance(node)
    node.querySelectorAll('[data-tip]').forEach(enhance)
  }
  scan(document.body)
  const observer = new MutationObserver(records => {
    if (anchor && !visible(anchor)) close()
    for (const record of records) {
      if (record.type === 'attributes' && record.target instanceof Element) {
        enhance(record.target)
        if (record.target === anchor) notify()
      }
      for (const node of record.addedNodes) if (node instanceof Element) scan(node)
    }
  })
  observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-tip', 'data-tip-variant'] })
  const target = (event: Event) => event.target instanceof Element ? event.target.closest('[data-tip]') : null
  document.addEventListener('pointerover', event => {
    if (!window.matchMedia('(hover: hover)').matches) return
    const el = target(event)
    if (!el || (event.relatedTarget instanceof Node && el.contains(event.relatedTarget))) return
    window.clearTimeout(hideTimer)
    window.clearTimeout(showTimer)
    showTimer = window.setTimeout(() => open(el), anchor ? 0 : 150)
  })
  document.addEventListener('pointerout', event => {
    const el = target(event)
    if (!el || (event.relatedTarget instanceof Node && el.contains(event.relatedTarget)) || pinned) return
    window.clearTimeout(showTimer)
    hideTimer = window.setTimeout(() => { if (anchor === el && !pinned) close() }, 80)
  })
  document.addEventListener('focusin', event => {
    const el = target(event)
    if (el?.matches(':focus-visible')) open(el)
  })
  document.addEventListener('focusout', event => { if (target(event) === anchor) close() })
  document.addEventListener('click', event => {
    const el = target(event)
    if (!el) { if (anchor) close(); return }
    if (anchor === el && pinned) close()
    else { open(el); pinned = true }
  }, true)
  document.addEventListener('keydown', event => { if (event.key === 'Escape' && anchor) close() })
  const follow = () => { if (anchor && !visible(anchor)) close() }
  window.addEventListener('scroll', follow, true)
  window.addEventListener('resize', follow)
  ;(window as unknown as { wbTooltip: { open: typeof open; close: typeof close; enhanced(el: Element): boolean } }).wbTooltip = { open, close, enhanced: el => registry.has(el) }
}

if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot, { once: true })
else boot()
