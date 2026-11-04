import * as React from 'react'
import { Tooltip as BaseTooltip } from '@base-ui/react/tooltip'
import { cn } from './lib/cn'

/**
 * 共享提示气泡：品牌底和浅底两种形态，固定定位并处理视口边界。
 * 声明式使用 TooltipTrigger；命令式提示桥通过 anchor 指定现有元素。
 * 提示层高于弹窗与下拉层、低于全局通知，不接管业务的点击钉住状态。
 */

/** 四个部件都按标准包一层，只为挂上 data-slot 标记（样式钩子与调试的锚点） */
function TooltipProvider(props: BaseTooltip.Provider.Props) {
  return <BaseTooltip.Provider data-slot='tooltip-provider' {...props} />
}

function Tooltip(props: BaseTooltip.Root.Props) {
  return <BaseTooltip.Root data-slot='tooltip' {...props} />
}

function TooltipTrigger(props: BaseTooltip.Trigger.Props) {
  return <BaseTooltip.Trigger data-slot='tooltip-trigger' {...props} />
}

type TooltipContentProps = Omit<React.ComponentProps<typeof BaseTooltip.Popup>, 'className'> & {
  className?: string
  /** 与锚点的距离，默认 8px */
  sideOffset?: number
  /** 锚定方向，默认 top（气泡在锚点上方） */
  side?: 'top' | 'right' | 'bottom' | 'left'
  anchor?: React.ComponentProps<typeof BaseTooltip.Positioner>['anchor']
  variant?: 'default' | 'plain'
}

function TooltipContent({ className, sideOffset = 8, side = 'top', anchor, variant = 'default', children, ...props }: TooltipContentProps) {
  return (
    <BaseTooltip.Portal>
      <BaseTooltip.Positioner anchor={anchor} positionMethod='fixed' collisionPadding={8} sideOffset={sideOffset} side={side} className='z-[90] data-anchor-hidden:hidden'>
        <BaseTooltip.Popup
          data-slot='tooltip-content'
          className={cn(
            'max-w-[min(320px,calc(100vw-16px))] rounded-md px-2.5 py-2 text-[11.5px] leading-[1.55] shadow-3',
            variant === 'plain' ? 'border border-border bg-raised text-foreground' : 'bg-primary text-primary-on',
            'break-words outline-none',
            'transition-[opacity,transform] duration-150 ease-out',
            'data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95',
            'data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95',
            className
          )}
          {...props}
        >
          {children}
        </BaseTooltip.Popup>
      </BaseTooltip.Positioner>
    </BaseTooltip.Portal>
  )
}

/** 气泡箭头：与气泡同色的 45° 小方块 */
function TooltipArrow({ className, ...props }: React.ComponentProps<typeof BaseTooltip.Arrow>) {
  return (
    <BaseTooltip.Arrow
      className={cn(
        'size-2 rotate-45 rounded-[1px] bg-primary',
        'data-[side=top]:-bottom-1 data-[side=bottom]:-top-1',
        className
      )}
      {...props}
    />
  )
}

export {
  Tooltip,
  TooltipProvider,
  TooltipTrigger,
  TooltipContent,
  TooltipArrow,
  type TooltipContentProps,
}
