import * as React from 'react'
import { cn } from './lib/cn'

/**
 * 多行文本域（标准形态，与 shadcn/ui 的 Textarea 一致）。
 *
 * 原生 `<textarea>` 的样式化封装：`data-slot="textarea"`、props 全透传。
 * 样式值取项目令牌：最小高 104px、10/12 内边距、等宽字体（配置类内容居多）、
 * 可纵向拉伸，与 ui/css/components.css 的 `textarea` 规则一致。
 */
function Textarea({ className, ...props }: React.ComponentProps<'textarea'>) {
  return (
    <textarea
      data-slot='textarea'
      className={cn(
        'min-h-26 w-full resize-y rounded-md border border-control-border bg-control px-3 py-2.5',
        'font-mono text-[12px] leading-[1.6] text-foreground outline-none max-[767px]:text-base',
        'transition-[border-color,box-shadow] duration-150 ease-out',
        'placeholder:text-muted-foreground',
        'hover:border-control-border-hover',
        'focus:border-primary focus:shadow-focus',
        'disabled:cursor-not-allowed disabled:bg-surface-3 disabled:text-muted-foreground',
        className
      )}
      {...props}
    />
  )
}

export { Textarea }
