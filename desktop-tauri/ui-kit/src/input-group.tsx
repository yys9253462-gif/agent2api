import * as React from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from './lib/cn'
import { Button } from './button'
import { Input } from './input'
import { Textarea } from './textarea'

/**
 * 输入框组 —— 带前缀 / 后缀的输入框。
 *
 * 组件划分、props 名字与行为全部照 shadcn/ui 的 InputGroup：
 *
 *   InputGroup
 *   ├── InputGroupInput / InputGroupTextarea   （内部控件，data-slot="input-group-control"）
 *   ├── InputGroupAddon                        （align: inline-start | inline-end | block-start | block-end）
 *   ├── InputGroupButton                       （size: xs | sm | icon-xs | icon-sm）
 *   └── InputGroupText                         （纯文字前缀，如 https:// 、$ ）
 *
 * 三条与标准一致的行为约定，改的时候别破坏：
 *
 * 1) addon 在 DOM 里**排在控件之后**，视觉位置交给 `align` 的 order-first / order-last ——
 *    这样 Tab 顺序是先控件后 addon（addon 里的按钮才不会被排在输入框前面）。
 * 2) addon 是 flex 子项，不是绝对定位；点击 addon 的空白处会聚焦输入框
 *    （内部按钮除外，它有自己的点击行为）。
 * 3) 边框、圆角、底色与焦点环都在**容器**上，内部控件用 InputGroupInput 抹掉自己的那套
 *    （否则会出现双层边框、双份内边距）。
 *
 * 样式值取项目令牌：容器 30px 高、控件描边、--r-md 圆角；addon 内衬 9px，
 * 让图标落在距容器边缘 10px 处 —— 与 ui/css 那套 `.input-affix` 的观感逐像素一致。
 */

function InputGroup({ className, ...props }: React.ComponentProps<'div'>) {
  return (
    <div
      data-slot='input-group'
      role='group'
      className={cn(
        'group/input-group relative flex h-[30px] w-full min-w-0 items-center rounded-md border border-control-border bg-control max-[767px]:h-10',
        'transition-[border-color,box-shadow] duration-150 ease-out',
        'outline-none',
        // 焦点环与描边跟着内部控件的 focus-visible 走（容器自己不是焦点元素）
        'has-[[data-slot=input-group-control]:focus-visible]:border-primary',
        'has-[[data-slot=input-group-control]:focus-visible]:shadow-focus',
        // 悬停加深描边：控件 hover 时容器跟着变，观感与单个输入框一致。
        // 必须排除 focus-visible —— 两条 has- 变体特异性相同，谁生效只看生成顺序，
        // 不排除的话鼠标停在聚焦的输入框上会显示 hover 色而不是主色。
        'has-[[data-slot=input-group-control]:hover:not(:focus-visible)]:border-control-border-hover',
        'has-[>textarea]:h-auto',
        // 前后 addon 占位后，控件在那一侧只需留一点间距
        'has-[>[data-align=inline-start]]:[&>[data-slot=input-group-control]]:pl-1.5',
        'has-[>[data-align=inline-end]]:[&>[data-slot=input-group-control]]:pr-1.5',
        // 上下 addon：容器转纵向，控件留出对应侧的内边距
        'has-[>[data-align=block-start]]:h-auto has-[>[data-align=block-start]]:flex-col',
        'has-[>[data-align=block-start]]:[&>[data-slot=input-group-control]]:pb-1.5',
        'has-[>[data-align=block-end]]:h-auto has-[>[data-align=block-end]]:flex-col',
        'has-[>[data-align=block-end]]:[&>[data-slot=input-group-control]]:pt-1.5',
        className
      )}
      {...props}
    />
  )
}

const inputGroupAddonVariants = cva(
  [
    'flex h-auto cursor-text items-center justify-center gap-1.5 select-none',
    'text-[12px] font-medium text-muted-foreground',
    'group-data-[disabled=true]/input-group:opacity-50',
  ],
  {
    variants: {
      align: {
        'inline-start': 'order-first pl-[9px] has-[>button]:-ml-1',
        'inline-end': 'order-last pr-[9px] has-[>button]:-mr-1',
        'block-start': 'order-first w-full justify-start px-2.5 pt-2 group-has-[>input]/input-group:pt-1.5',
        'block-end': 'order-last w-full justify-start px-2.5 pb-2 group-has-[>input]/input-group:pb-1.5',
      },
    },
    defaultVariants: { align: 'inline-start' },
  }
)

type InputGroupAddonProps = React.ComponentProps<'div'> &
  VariantProps<typeof inputGroupAddonVariants>

function InputGroupAddon({ className, align = 'inline-start', ...props }: InputGroupAddonProps) {
  return (
    <div
      role='group'
      data-slot='input-group-addon'
      data-align={align}
      className={cn(inputGroupAddonVariants({ align }), className)}
      onClick={event => {
        // 点 addon 的空白处聚焦输入框；内部按钮有自己的点击行为，放行
        if ((event.target as HTMLElement).closest('button')) return
        event.currentTarget.parentElement
          ?.querySelector<HTMLInputElement>('input')
          ?.focus()
      }}
      {...props}
    />
  )
}

const inputGroupButtonVariants = cva('flex items-center shadow-none', {
  variants: {
    size: {
      xs: 'h-6 gap-1 rounded-xs px-1.5',
      sm: 'h-8 gap-1.5 rounded-sm px-2.5',
      'icon-xs': 'size-6 rounded-xs p-0',
      'icon-sm': 'size-8 rounded-sm p-0',
    },
  },
  defaultVariants: { size: 'xs' },
})

type InputGroupButtonProps = Omit<React.ComponentProps<typeof Button>, 'size'> &
  VariantProps<typeof inputGroupButtonVariants> & {
    type?: 'button' | 'submit' | 'reset'
  }

function InputGroupButton({
  className,
  type = 'button',
  variant = 'ghost',
  size = 'xs',
  ...props
}: InputGroupButtonProps) {
  return (
    <Button
      type={type}
      data-size={size}
      variant={variant}
      className={cn(inputGroupButtonVariants({ size }), className)}
      {...props}
    />
  )
}

function InputGroupText({ className, ...props }: React.ComponentProps<'span'>) {
  return (
    <span
      className={cn(
        'flex items-center gap-1.5 text-[12px] text-muted-foreground [&_svg]:pointer-events-none',
        className
      )}
      {...props}
    />
  )
}

function InputGroupInput({ className, ...props }: React.ComponentProps<'input'>) {
  return (
    <Input
      data-slot='input-group-control'
      className={cn(
        // 抹掉 Input 自带的那套：边框、圆角、底色、焦点环都归容器
        'flex-1 rounded-none border-0 bg-transparent shadow-none focus:border-0 focus:shadow-none',
        className
      )}
      {...props}
    />
  )
}

function InputGroupTextarea({ className, ...props }: React.ComponentProps<'textarea'>) {
  return (
    <Textarea
      data-slot='input-group-control'
      className={cn(
        'flex-1 resize-none rounded-none border-0 bg-transparent py-2 shadow-none focus:border-0 focus:shadow-none',
        className
      )}
      {...props}
    />
  )
}

export {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupText,
  InputGroupInput,
  InputGroupTextarea,
}
