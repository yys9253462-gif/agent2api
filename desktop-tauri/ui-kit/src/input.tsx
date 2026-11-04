import * as React from 'react'
import { Input as InputPrimitive } from '@base-ui/react/input'
import { cn } from './lib/cn'

/**
 * 输入框（标准形态，与 shadcn/ui 的 Input 一致）。
 *
 * 就是原生 `<input>` 的样式化封装：`data-slot="input"`、props 全透传，
 * 不额外发明 prop。要带图标 / 文字 / 按钮的前后缀，用 InputGroup —— 那是 shadcn
 * 给这类需求准备的标准组件（本组件此前有过一个自造的 `icon` prop，已移除）。
 *
 * 样式值取项目令牌：30px 高、控件底、1px 控件描边、12.5px 字号，hover 加深描边、
 * focus 换主色描边并压焦点环 —— 对齐 ui/css/components.css 的 `input[type=…]` 一族。
 *
 * 用 Base UI 的 Input 而不是裸 `<input>`：它带 data-disabled / data-valid 等
 * 状态属性，将来接 Field 做校验时不用换组件。
 */

function Input({ className, type, ref, ...props }: React.ComponentProps<'input'>) {
  if (type === 'file' || type === 'hidden') {
    return <input ref={ref} type={type} data-slot='input' className={cn(className)} {...props} />
  }
  return (
    <InputPrimitive
      ref={ref as React.Ref<HTMLElement>}
      type={type}
      data-slot='input'
      className={cn(
        'h-[30px] w-full min-w-0 rounded-md border border-control-border bg-control px-2.5',
        'text-[12.5px] text-foreground outline-none max-[767px]:h-10 max-[767px]:text-base',
        'transition-[border-color,box-shadow] duration-150 ease-out',
        'placeholder:text-muted-foreground',
        'hover:border-control-border-hover',
        'focus:border-primary focus:shadow-focus',
        'disabled:pointer-events-none disabled:cursor-not-allowed disabled:bg-surface-3 disabled:text-muted-foreground',
        'file:inline-flex file:border-0 file:bg-transparent file:text-foreground',
        className
      )}
      {...props}
    />
  )
}

export { Input }
