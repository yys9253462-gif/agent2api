import * as React from 'react'
import { useRender } from '@base-ui/react/use-render'
import { Badge } from './badge'
import { Button } from './button'
import { Checkbox } from './checkbox'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from './collapsible'
import { Empty } from './feedback'
import { Input } from './input'
import { Label } from './label'
import { StatusDot } from './status-dot'
import { Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow } from './table'
import { Textarea } from './textarea'
import { cn } from './lib/cn'

const tags = new Set(['a', 'article', 'b', 'blockquote', 'br', 'code', 'del', 'div', 'em', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'hr', 'i', 'img', 'kbd', 'li', 'ol', 'p', 'pre', 's', 'section', 'small', 'span', 'strong', 'sub', 'sup', 'ul', 'col', 'colgroup'])
const controls: Record<string, React.ElementType> = {
  button: Button, input: Input, label: Label, textarea: Textarea,
  table: Table, thead: TableHeader, tbody: TableBody, tfoot: TableFooter,
  tr: TableRow, th: TableHead, td: TableCell, caption: TableCaption,
}
const safeStyles = new Set(['text-align', 'white-space', 'word-break', 'overflow-wrap', 'font-weight', 'font-style', 'font-size', 'line-height', 'color', 'background-color', 'width', 'max-width', 'min-width', 'height', 'padding', 'margin', 'display', 'gap', 'flex', 'grid-column'])

function safeUrl(value: string, image = false): string | undefined {
  if (image && /^data:image\/(?:png|jpe?g|gif|webp|avif);base64,/i.test(value)) return value
  if (/^(?:https?:\/\/|\/[^/]|\.\.?\/|#)/i.test(value)) return value
  return undefined
}

function attributes(element: Element): Record<string, unknown> {
  const props: Record<string, unknown> = {}
  for (const attr of element.attributes) {
    if (attr.name === 'class') props.className = attr.value
    else if (attr.name === 'colspan') props.colSpan = Number(attr.value) || 1
    else if (attr.name === 'rowspan') props.rowSpan = Number(attr.value) || 1
    else if (attr.name === 'target' && ['_blank', '_self'].includes(attr.value)) props.target = attr.value
    else if (attr.name === 'loading' && ['lazy', 'eager'].includes(attr.value)) props.loading = attr.value
    else if (['title', 'id', 'role', 'alt', 'scope', 'type', 'placeholder'].includes(attr.name)) props[attr.name] = attr.value
    else if (/^(aria-|data-)/.test(attr.name)) props[attr.name] = attr.value
    else if (attr.name === 'href') props.href = safeUrl(attr.value)
    else if (attr.name === 'src') props.src = safeUrl(attr.value, true)
  }
  if (element.tagName === 'A') props.rel = 'noopener noreferrer'
  if (element instanceof HTMLElement) {
    const style: Record<string, string> = {}
    for (const name of element.style) {
      if (!safeStyles.has(name)) continue
      const value = element.style.getPropertyValue(name)
      if (/url\(|expression\(/i.test(value)) continue
      style[name.replace(/-([a-z])/g, (_, char: string) => char.toUpperCase())] = value
    }
    if (Object.keys(style).length) props.style = style
  }
  return props
}

function content(nodes: NodeListOf<ChildNode> | ChildNode[], prefix = ''): React.ReactNode[] {
  return Array.from(nodes).map((node, index) => {
    const key = `${prefix}${index}`
    if (node.nodeType === Node.TEXT_NODE) return node.textContent
    if (!(node instanceof Element)) return null
    const tag = node.tagName.toLowerCase()
    if (['script', 'style', 'iframe', 'object', 'embed'].includes(tag)) return null
    const props = attributes(node)
    const children = content(node.childNodes, `${key}-`)
    if (tag === 'details') {
      const summary = Array.from(node.children).find(child => child.tagName === 'SUMMARY')
      return <Collapsible key={key} defaultOpen={node.hasAttribute('open')} {...props}>
        <CollapsibleTrigger className={cn('h-auto w-full justify-start whitespace-normal px-0 text-left', summary?.className)}>
          <span className='shrink-0 transition-transform in-data-panel-open:rotate-90' aria-hidden='true'>▸</span>
          {summary ? content(summary.childNodes, `${key}-summary-`) : '…'}
        </CollapsibleTrigger>
        <CollapsibleContent keepMounted>{content(Array.from(node.childNodes).filter(child => child !== summary), `${key}-body-`)}</CollapsibleContent>
      </Collapsible>
    }
    if (tag === 'span' && node.classList.contains('badge')) {
      const variant = node.classList.contains('ok') || node.classList.contains('phase-streaming') ? 'success' : node.classList.contains('bad') ? 'destructive'
        : node.classList.contains('warn') ? 'warning' : node.classList.contains('brand') ? 'brand' : 'outline'
      return <Badge key={key} variant={variant} shape={node.classList.contains('tag') ? 'tag' : 'pill'} {...props}>{children}</Badge>
    }
    if (tag === 'span' && node.classList.contains('req-live-dot')) return <StatusDot key={key} pulse className='size-[5px] bg-current' />
    if (tag === 'div' && node.classList.contains('empty')) return <Empty key={key} {...props}>{children}</Empty>
    if (tag === 'input' && node.getAttribute('type') === 'checkbox') return <Checkbox key={key} checked={node.hasAttribute('checked')} readOnly />
    if (tag === 'table') return <Table key={key} container={false} {...props}>{children}</Table>
    if (controls[tag]) return React.createElement(controls[tag], { key, ...props }, tag === 'input' ? undefined : children)
    if (!tags.has(tag)) return <React.Fragment key={key}>{children}</React.Fragment>
    if (['br', 'hr', 'img', 'col'].includes(tag)) return React.createElement(tag, { key, ...props })
    return React.createElement(tag, { key, ...props }, children)
  })
}

function HtmlContent({ html, className, render, ...props }: useRender.ComponentProps<'div'> & { html: string }) {
  const children = React.useMemo(() => {
    if (!html) return []
    const document = new DOMParser().parseFromString(html, 'text/html')
    return content(document.body.childNodes)
  }, [html])
  return useRender({ defaultTagName: 'div', render, props: { ...props, 'data-slot': 'html-content', className: cn(className), children } })
}

export { HtmlContent }
