import * as React from 'react'
import { Radio } from '@base-ui/react/radio'
import { RadioGroup as RadioGroupPrimitive } from '@base-ui/react/radio-group'
import { cn } from './lib/cn'

/**
 * 单选组（标准形态，与 shadcn/ui 的 RadioGroup 一致：`data-slot="radio-group"` /
 * `data-slot="radio-group-item"` / `data-slot="radio-group-indicator"`）。
 *
 * 自绘圆钮（不再是原生 `<input type=radio>`）：原生圆钮的观感由浏览器决定，
 * accent-color 只能改填充色。自绘后尺寸固定 14px（与 Checkbox 同档）、选中填主色 +
 * 白色圆点，与既有 `.clear-mode input[type="radio"]` 的观感一致。
 *
 * 用法（受控）：
 *   <RadioGroup value={v} onValueChange={setV}>
 *     <RadioGroupItem value="a" />
 *     <RadioGroupItem value="b" />
 *   </RadioGroup>
 *
 * 键盘交互、roving tabindex、`role="radiogroup"` 由 Base UI 负责；
 * 方向键在组内移动即选中，与原生单选组同款行为。
 *
 * 注意：Radio.Root 渲染的是 `<span>` + 旁边的隐藏 `<input>`，不是 `<button>`。
 * 别改成 `render={<button />}` —— 那会覆盖 CompositeItem 注册的方向键处理，
 * 组内方向键会失效（详见 segmented-control.tsx 的同一处说明）。
 */

type RadioGroupProps<Value = string> = Omit<
  React.ComponentProps<typeof RadioGroupPrimitive<Value>>,
  'className'
> & { className?: string }

function RadioGroup<Value = string>({ className, ...props }: RadioGroupProps<Value>) {
  return (
    <RadioGroupPrimitive<Value>
      data-slot='radio-group'
      className={cn('flex flex-col gap-2', className)}
      {...props}
    />
  )
}

type RadioGroupItemProps = Omit<React.ComponentProps<typeof Radio.Root>, 'className'> & {
  className?: string
}

function RadioGroupItem({ className, children, ...props }: RadioGroupItemProps) {
  return (
    <Radio.Root
      data-slot='radio-group-item'
      className={cn(
        'relative inline-flex size-3.5 flex-none cursor-pointer items-center justify-center rounded-full border border-control-border bg-control',
        'outline-none transition-colors duration-150 ease-out',
        'hover:border-control-border-hover',
        'data-checked:border-primary data-checked:bg-primary',
        'focus-visible:shadow-focus',
        'data-disabled:cursor-not-allowed data-disabled:opacity-45',
        className
      )}
      {...props}
    >
      {children}
      <Radio.Indicator
        data-slot='radio-group-indicator'
        className='grid place-content-center data-unchecked:hidden'
      >
        {/* 选中态的白点：主色实底上的一枚圆点，与原生单选钮同观感 */}
        <span className='block size-[5px] rounded-full bg-primary-foreground' />
      </Radio.Indicator>
    </Radio.Root>
  )
}

export { RadioGroup, RadioGroupItem, type RadioGroupProps, type RadioGroupItemProps }
