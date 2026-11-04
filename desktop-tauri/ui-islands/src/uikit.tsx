/**
 * 组件库的**跨文档**出口（`window.wbUiKit`）。
 *
 * ── 为什么需要这个文件 ──────────────────────────────────────
 * 插件页（签到中心这类）跑在 iframe 里，是独立文档：它既读不到宿主已加载的
 * React，也拿不到组件库的模块实例。于是它只能自绘一套控件 —— 而自绘的控件
 * 与组件库必然漂移（尺寸差一档、圆角差半档、悬停过渡不同），表现成「迁移过来
 * 的页面长得不一样」。这个问题靠「抄数值」是修不干净的：组件库每改一次令牌，
 * 插件页就落后一次。
 *
 * 这个文件把组件库连同 React 运行时一起挂到 `window` 上，插件页在 iframe 里
 * `<script src="…/islands/ui.js">` 之后即可 `const { Badge, Button } = window.wbUiKit`
 * 直接用 —— 于是**同一份实现、同一份样式、同一套令牌**，天然不会漂移。
 *
 * ── 代价（选这条路的已知取舍）────────────────────────────────
 * 插件页从此依赖网关的 islands 产物：
 *   · 插件包不能自带这份 js（它会带一份 React，两份调度器互不认识）；
 *   · 网关升级改了组件库 API 时，已发布的插件页可能失配。
 * 后者由「组件库保持向后兼容的 props」这条约定兜住（与 islands 内部各岛之间
 * 的约定相同）。插件协议里 `protocol` 字段仍是版本闸门：不兼容时网关会拒绝
 * 加载旧包。
 *
 * ── 为什么挂在 window 上而不是做成 ESM 导出 ─────────────────
 * `ui.js` 是 iife 产物（见 vite.config.ts 的说明：既有 60 多个前端脚本都是
 * 经典脚本、靠加载顺序定依赖，混进 module 会多一层时序问题）。iife 没有
 * export，唯一的对外通道就是全局对象。
 */

import * as React from 'react'
import { createRoot } from 'react-dom/client'
import { flushSync } from 'react-dom'
import {
  Badge, Button, Checkbox, Input, Label, Progress, SegmentedControl, Spinner, Switch,
  Select, SelectTrigger, SelectValue, SelectContent, SelectItem,
  SelectGroup, SelectLabel, SelectSeparator, Card, CardHeader, CardContent, CardFooter,
  Collapsible, CollapsibleTrigger, CollapsibleContent, HtmlContent, Toaster, toastManager,
} from '@ui'

declare global {
  interface Window {
    /**
     * 组件库 + React 的跨文档出口（插件页 iframe 用）。
     *
     * 只暴露插件页实际用得到的那些 —— 全量导出会把组件库的内部结构变成
     * 对外契约，而它本来可以自由重构。需要新组件时在这里加一行，
     * 顺带也就记录下了「谁在用」。
     */
    wbUiKit?: {
      React: typeof React
      createRoot: typeof createRoot
      /** 同步提交：需要在 render() 之后立刻拿到 DOM 节点时用（见插件页的 textInput） */
      flushSync: typeof flushSync
      Badge: typeof Badge
      Button: typeof Button
      Checkbox: typeof Checkbox
      Input: typeof Input
      Label: typeof Label
      Progress: typeof Progress
      SegmentedControl: typeof SegmentedControl
      Spinner: typeof Spinner
      Switch: typeof Switch
      Card: typeof Card
      CardHeader: typeof CardHeader
      CardContent: typeof CardContent
      CardFooter: typeof CardFooter
      Collapsible: typeof Collapsible
      CollapsibleTrigger: typeof CollapsibleTrigger
      CollapsibleContent: typeof CollapsibleContent
      HtmlContent: typeof HtmlContent
      Toaster: typeof Toaster
      toastManager: typeof toastManager
      /** Select 是一组（Root / Trigger / Value / Content / Item …），
       *  只给 Root 用不了 —— 调用方必须能拼出完整的组合 */
      Select: {
        Root: typeof Select
        Trigger: typeof SelectTrigger
        Value: typeof SelectValue
        Content: typeof SelectContent
        Item: typeof SelectItem
        Group: typeof SelectGroup
        Label: typeof SelectLabel
        Separator: typeof SelectSeparator
      }
    }
  }
}

window.wbUiKit = {
  React,
  createRoot,
  flushSync,
  Badge,
  Button,
  Checkbox,
  Input,
  Label,
  Progress,
  SegmentedControl,
  Spinner,
  Switch,
  Card, CardHeader, CardContent, CardFooter, Collapsible, CollapsibleTrigger, CollapsibleContent,
  HtmlContent, Toaster, toastManager,
  Select: {
    Root: Select,
    Trigger: SelectTrigger,
    Value: SelectValue,
    Content: SelectContent,
    Item: SelectItem,
    Group: SelectGroup,
    Label: SelectLabel,
    Separator: SelectSeparator,
  },
}
