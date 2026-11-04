import * as React from 'react'
import { Switch as BaseSwitch } from '@base-ui/react/switch'
import { cn } from './lib/cn'

/**
 * 开关。
 *
 * 视觉对齐 ui/css/components.css 的 `.switch`：36×21 的胶囊轨道 + 17px 圆形滑块，
 * 选中时轨道填主色、滑块右移 15px。滑块带一点回弹（cubic-bezier(.3,1.4,.6,1)），
 * 比线性更像物理开关 —— 这是既有观感的一部分，别改成 ease。
 *
 * 交互交给 Base UI 的 Switch：它是真正的 `<button role="switch">`（不是藏起来的
 * 原生 checkbox），键盘 Space/Enter、aria-checked 播报都由它负责。
 * 原来那套「原生 input + 相邻兄弟选择器画轨道」的写法在语义与触摸上都更差。
 *
 * 用法：`<Switch checked={on} onCheckedChange={setOn} />`，要带文字就套一层
 * `<label>`（组件库不预设文案排版）。
 */

type SwitchProps = Omit<React.ComponentProps<typeof BaseSwitch.Root>, 'className'> & {
  className?: string
  /**
   * 尺寸档。`sm` 是 24×14 的小号，给药丸（chip）内部用 —— 药丸本身只有 22px 高，
   * 标准档（36×21）塞进去会把 chip 撑高，而 chip 的高度是表格行高的一部分。
   */
  size?: 'sm' | 'default' | 'touch'
  variant?: 'default' | 'success'
}

function Switch({ className, size = 'default', variant = 'default', ...props }: SwitchProps) {
  const small = size === 'sm'
  return (
    <BaseSwitch.Root
      data-slot='switch'
      className={cn(
        'relative inline-flex flex-none cursor-pointer items-center rounded-pill border-0 bg-border-strong p-0',
        'transition-colors duration-[180ms] ease-out',
        variant === 'success' ? 'data-checked:bg-success' : 'data-checked:bg-primary',
        'focus-visible:shadow-focus focus-visible:outline-none',
        'data-disabled:cursor-not-allowed data-disabled:opacity-50',
        small ? 'h-[14px] w-6' : size === 'touch' ? 'h-6 w-10' : 'h-[21px] w-9',
        className
      )}
      {...props}
    >
      <BaseSwitch.Thumb
        data-slot='switch-thumb'
        className={cn(
          'block rounded-full bg-primary-on shadow-1',
          'transition-transform duration-[180ms] [transition-timing-function:cubic-bezier(.3,1.4,.6,1)]',
          small
            ? 'size-[10px] translate-x-0.5 data-checked:translate-x-[10px]'
            : size === 'touch'
              ? 'size-5 translate-x-0.5 data-checked:translate-x-[18px]'
              : 'size-[17px] translate-x-0.5 data-checked:translate-x-[17px]'
        )}
      />
    </BaseSwitch.Root>
  )
}

export { Switch, type SwitchProps }
