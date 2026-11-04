import * as React from 'react'
import { Progress as ProgressPrimitive } from '@base-ui/react/progress'
import { cn } from './lib/cn'

/**
 * 进度条（标准形态，与 shadcn/ui 的 Progress 一致）。
 *
 * 部件划分照标准：Progress 是容器，默认自带一条 ProgressTrack + ProgressIndicator
 * （传 value 就能用）；要自定义结构或加百分比文字时，用 ProgressLabel /
 * ProgressValue，或自己组装 Track 与 Indicator。
 *
 * 样式值取项目令牌：6px 高胶囊、轨道取 surface-3、条体主色、宽度变化 0.3s 缓动 ——
 * 对齐 ui/css/components.css 的 `.progress`。
 *
 * 用 Base UI 的 Progress 而不是自绘 div：它补上 `role="progressbar"` 与
 * `aria-valuenow/valuemin/valuemax`，读屏能播报百分比。`value` 传 null 表示
 * 不确定进度（会省略 aria-valuenow）。
 */

function Progress({
  className,
  children,
  value,
  ...props
}: ProgressPrimitive.Root.Props) {
  return (
    <ProgressPrimitive.Root
      value={value}
      data-slot='progress'
      className={cn('flex w-full flex-wrap items-center gap-3', className)}
      {...props}
    >
      {children}
      <ProgressTrack>
        <ProgressIndicator />
      </ProgressTrack>
    </ProgressPrimitive.Root>
  )
}

function ProgressTrack({ className, ...props }: ProgressPrimitive.Track.Props) {
  return (
    <ProgressPrimitive.Track
      data-slot='progress-track'
      className={cn(
        'relative flex h-1.5 w-full items-center overflow-x-hidden overflow-y-hidden rounded-pill bg-surface-3',
        className
      )}
      {...props}
    />
  )
}

function ProgressIndicator({ className, ...props }: ProgressPrimitive.Indicator.Props) {
  return (
    <ProgressPrimitive.Indicator
      data-slot='progress-indicator'
      className={cn('h-full rounded-[inherit] bg-primary transition-[width] duration-300 ease-out data-[indeterminate]:w-1/3 data-[indeterminate]:animate-pulse', className)}
      {...props}
    />
  )
}

function ProgressLabel({ className, ...props }: ProgressPrimitive.Label.Props) {
  return (
    <ProgressPrimitive.Label
      data-slot='progress-label'
      className={cn('text-[11.5px] text-muted-foreground', className)}
      {...props}
    />
  )
}

function ProgressValue({ className, ...props }: ProgressPrimitive.Value.Props) {
  return (
    <ProgressPrimitive.Value
      data-slot='progress-value'
      className={cn('ml-auto text-[11.5px] tabular-nums text-muted-foreground', className)}
      {...props}
    />
  )
}

export { Progress, ProgressTrack, ProgressIndicator, ProgressLabel, ProgressValue }
