/**
 * 账号设置弹窗的「查询设置」段：**每账号**的自动余额查询间隔与「余额不足处理」。
 *
 * 独立成文件的原因：accounts-dialogs.tsx 已超过单文件行数约定，这一段是完整
 * 的功能块（状态草稿 + 校验 + 渲染），放进去只会继续膨胀。
 *
 * ── 全局任务已退役，间隔是每账号自己的 ────────────────────────
 * 全局「定时查询积分」任务删除后，余额查询按账号各自的间隔到期触发
 * （`core::usage_query` 的心跳循环），本段就是那个间隔与「余额不足时怎么办」
 * 的唯一配置入口。保存走 PATCH /api/accounts 的 `usageQuery` / `lowBalance`
 * 两个键（后端 `apply_patch` 归一化校验），界面的单位选择（秒 / 分钟 / 小时）
 * 只是输入辅助，落库一律是**秒**。
 */

import * as React from 'react'
import {
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Switch,
} from '@ui'
import { type AccountRecord, type UsageEntry } from './accounts-shared'
import {
  lowBalanceOf, usageQueryOf,
  USAGE_QUERY_MAX_SECONDS, USAGE_QUERY_MIN_SECONDS,
} from './accounts-domain'
import { usageEntryOf } from './accounts-data'

/** 间隔输入的单位（界面辅助；落库统一换算成秒） */
type IntervalUnit = 'seconds' | 'minutes' | 'hours'

const UNIT_MS: Record<IntervalUnit, number> = { seconds: 1, minutes: 60, hours: 3600 }

/** 「查询设置」段的草稿：数字输入框以字符串中转（输入中途允许为空 / 非法） */
export type UsageDraft = {
  enabled: boolean
  intervalInput: string
  unit: IntervalUnit
  mode: 'off' | 'skip' | 'disable'
  thresholdInput: string
}

/** 把秒数拆成「数字 + 最大可整除的单位」（900 秒 → 15 分钟）；0 → 5 分钟缺省 */
function splitInterval(seconds: number): { value: string; unit: IntervalUnit } {
  if (seconds > 0 && seconds % 3600 === 0) return { value: String(seconds / 3600), unit: 'hours' }
  if (seconds > 0 && seconds % 60 === 0) return { value: String(seconds / 60), unit: 'minutes' }
  if (seconds > 0) return { value: String(seconds), unit: 'seconds' }
  return { value: '5', unit: 'minutes' }
}

/** 账号现有配置 → 草稿（读侧缺省：开启、1 分钟、跳过、阈值 1 —— 见 accounts-domain） */
export function usageDraftOf(account: AccountRecord | null | undefined): UsageDraft {
  const { enabled, interval } = usageQueryOf(account)
  // 关着的时候间隔输入给个 5 分钟的落点值：重新拨开时不用面对空输入框
  const intervalDraft = splitInterval(enabled ? interval : 0)
  const { mode, threshold } = lowBalanceOf(account)
  return {
    enabled,
    intervalInput: intervalDraft.value,
    unit: intervalDraft.unit,
    mode,
    thresholdInput: threshold > 0 ? String(threshold) : '',
  }
}

export type UsageChanges =
  | { error: string }
  | { patch: { usageQuery?: { enabled: boolean; interval: number }; lowBalance?: { mode: string; threshold: number } } }

/**
 * 读本段的改动：什么都没改 → null（不提交，与 dialogs 里各段的读取函数同一
 * 形态）；校验不过 → `{error}`；有改动 → `{patch}`（只带真变了的那一块）。
 * 校验先于落库：后端会对同一规则 400，但先在前端挡一次省一次保存失败。
 */
export function readUsageChanges(account: AccountRecord, draft: UsageDraft): UsageChanges | null {
  const seconds = Math.round(Number(draft.intervalInput) * UNIT_MS[draft.unit])
  if (draft.enabled) {
    if (!Number.isFinite(Number(draft.intervalInput)) || Number(draft.intervalInput) <= 0) {
      return { error: '请填写查询间隔' }
    }
    if (seconds < USAGE_QUERY_MIN_SECONDS || seconds > USAGE_QUERY_MAX_SECONDS) {
      return { error: `查询间隔必须是 ${USAGE_QUERY_MIN_SECONDS} 秒 ~ 24 小时` }
    }
  }
  const patch: {
    usageQuery?: { enabled: boolean; interval: number }
    lowBalance?: { mode: string; threshold: number }
  } = {}
  const current = usageQueryOf(account)
  // 间隔输入框只在开启时渲染：关闭状态下的 intervalInput 是挂载时的旧值，
  // 不拿它当改动（否则未配置过的账号保存一次就会凭空写进一份缺省间隔）
  if (draft.enabled !== current.enabled || (draft.enabled && seconds !== current.interval)) {
    patch.usageQuery = { enabled: draft.enabled, interval: draft.enabled ? seconds : current.interval }
  }
  const mode = draft.mode
  if (mode !== 'off') {
    const threshold = Number(draft.thresholdInput)
    if (!Number.isFinite(threshold) || threshold <= 0) {
      return { error: '余额阈值必须是大于 0 的数字（与余额列同一数字口径）' }
    }
    const currentLow = lowBalanceOf(account)
    if (mode !== currentLow.mode || threshold !== currentLow.threshold) {
      patch.lowBalance = { mode, threshold }
    }
  } else if (lowBalanceOf(account).mode !== 'off') {
    patch.lowBalance = { mode: 'off', threshold: 0 }
  }
  if (!patch.usageQuery && !patch.lowBalance) return null
  return { patch }
}

/** 余额列的当前读数（给阈值输入当参照）；判不出 / 没查过给空串 */
function currentBalanceText(account: AccountRecord): string {
  const entry: UsageEntry = usageEntryOf(account)
  if (!entry || typeof entry !== 'object') return ''
  const data = entry as Record<string, unknown>
  if (data.unlimited) return '当前可用 ∞（不参与余额不足判定）'
  if (typeof data.totalLeft === 'number') return `当前可用 ${data.totalLeft}`
  const available = Number(data.available)
  if (Number.isFinite(available)) {
    const unit = String(data.unit || '')
    return unit ? `当前可用 ${available} ${unit}` : `当前可用 ${available}`
  }
  return ''
}

/** 「查询设置」段（受控组件：草稿在弹窗手里，保存时由 readUsageChanges 汇总） */
export function UsageSettingsSection({
  account, draft, onChange,
}: {
  account: AccountRecord
  draft: UsageDraft
  onChange: (next: UsageDraft) => void
}) {
  const patch = (next: Partial<UsageDraft>): void => onChange({ ...draft, ...next })
  const balanceText = currentBalanceText(account)
  return (
    <>
      <div className='field-row mt-2.5'>
        <Label className='inline-flex cursor-pointer items-center gap-2.5 font-normal'>
          <Switch checked={draft.enabled} onCheckedChange={next => patch({ enabled: next === true })}
            aria-label='自动查询余额' />
          <span className='text-xs text-subtle'>自动查询余额（按下面的间隔自动刷新这一行的读数）</span>
        </Label>
      </div>
      {draft.enabled ? (
        <div className='field-row mt-2.5'>
          <label htmlFor='account-usage-interval'>查询间隔</label>
          <Input id='account-usage-interval' type='number' min={1} step={1}
            className='max-w-[110px]' value={draft.intervalInput}
            onChange={event => patch({ intervalInput: event.currentTarget.value })} />
          <Select value={draft.unit} onValueChange={value => patch({ unit: value as IntervalUnit })}>
            <SelectTrigger className='w-[92px]' aria-label='间隔单位'>
              <SelectValue>{draft.unit === 'hours' ? '小时' : draft.unit === 'minutes' ? '分钟' : '秒'}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='seconds'>秒</SelectItem>
              <SelectItem value='minutes'>分钟</SelectItem>
              <SelectItem value='hours'>小时</SelectItem>
            </SelectContent>
          </Select>
          <span className='detail'>30 秒 ~ 24 小时；手动查询会顺延下一轮</span>
        </div>
      ) : null}
      <div className='field-row mt-2.5'>
        <label htmlFor='account-lowbalance-mode'>余额不足时</label>
        <Select value={draft.mode} onValueChange={value => patch({ mode: value as UsageDraft['mode'] })}>
          <SelectTrigger id='account-lowbalance-mode' className='min-w-[220px]'>
            <SelectValue>
              {draft.mode === 'skip' ? '跳过该账号（余额回升自动恢复）'
                : draft.mode === 'disable' ? '禁用该账号（需手动重新启用）'
                : '不处理'}
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value='off'>不处理</SelectItem>
            <SelectItem value='skip'>跳过该账号（余额回升自动恢复）</SelectItem>
            <SelectItem value='disable'>禁用该账号（需手动重新启用）</SelectItem>
          </SelectContent>
        </Select>
      </div>
      {draft.mode !== 'off' ? (
        <div className='field-row mt-2.5'>
          <label htmlFor='account-lowbalance-threshold'>余额阈值</label>
          <Input id='account-lowbalance-threshold' type='number' min={0} step={1}
            className='max-w-[140px]' value={draft.thresholdInput}
            onChange={event => patch({ thresholdInput: event.currentTarget.value })} />
          <span className='detail'>
            {balanceText ? `${balanceText}；` : ''}低于该数字即判定不足（与余额列同一数字口径，严格小于才触发）
          </span>
        </div>
      ) : null}
    </>
  )
}
