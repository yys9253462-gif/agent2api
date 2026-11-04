import {
  errorMessage, shared, toast, type AccountRecord, type TokenReading,
} from './accounts-shared'
import { supportsCreditDetail, supportsUsage } from './accounts-domain'
import { allAccounts, bump, findAccount, getStore, patch } from './accounts-store'
import { adoptLimiterSnapshot } from './accounts-limiter-model'
import { adoptAccountSorting, refreshAccountSorting } from './accounts-sort'
import { NOT_CONFIGURED_CODE, putUsage, putUsageRow, usageEntries, usageEntryOf, usageFailureOf } from './accounts-usage-cache'
import { t } from '../i18n'

export { NOT_CONFIGURED_CODE, pruneUsageCache, usageEntries, usageEntryOf, usageFailureOf, usageFailureStatusOf, usageUpdatedAtOf } from './accounts-usage-cache'

const tokenUsageMap = new Map<string, TokenReading[]>()
let lastTokenAt = 0
let lastSnapshotAt = 0
let shapeRepairStarted = false

export function tokenUsageOf(account: AccountRecord | null | undefined): TokenReading[] {
  return account ? tokenUsageMap.get(account.id) || [] : []
}

function applyTokenUsage(
  snapshot: { tokenAt?: unknown; tokenUsage?: Record<string, unknown> } | null | undefined,
): boolean {
  const tokenAt = Number(snapshot?.tokenAt) || 0
  if (!tokenAt || tokenAt === lastTokenAt) return false
  const usage = snapshot?.tokenUsage
  if (!usage || typeof usage !== 'object') return false
  lastTokenAt = tokenAt
  const fresh = new Map<string, TokenReading[]>()
  for (const [id, rows] of Object.entries(usage)) {
    if (!Array.isArray(rows)) continue
    const readings: TokenReading[] = []
    for (const row of rows) {
      if (!row || typeof row !== 'object') continue
      const kind = Number((row as Record<string, unknown>).kind)
      const windowStart = Number((row as Record<string, unknown>).windowStart)
      const used = Number((row as Record<string, unknown>).used)
      if (!Number.isFinite(kind) || kind < 0) continue
      if (!Number.isFinite(windowStart) || windowStart <= 0) continue
      if (!Number.isFinite(used) || used < 0) continue
      readings.push({ kind, windowStart, used })
    }
    fresh.set(id, readings)
  }
  const changed = fresh.size !== tokenUsageMap.size
    || [...fresh].some(([id, readings]) => {
      const current = tokenUsageMap.get(id)
      return !current || current.length !== readings.length
        || readings.some((row, index) => row.used !== current[index]?.used
          || row.windowStart !== current[index]?.windowStart)
    })
  tokenUsageMap.clear()
  for (const [id, readings] of fresh) tokenUsageMap.set(id, readings)
  return changed
}

/** 批量只查有余额能力且凭证完整的账号；禁用不影响余额查询。 */
function batchUsageTargets(): AccountRecord[] {
  return allAccounts().filter(account => supportsUsage(account) && account.available !== false)
}

/** 行级时间戳优先，避免其他账号的查询时间覆盖这条失败结论的时刻。 */
export function applyBalances(
  balances: { results?: Array<Record<string, unknown>>; at?: unknown } | null | undefined,
): number {
  const rows = Array.isArray(balances?.results) ? balances.results : []
  const at = Number(balances?.at) || Date.now()
  let applied = 0
  for (const row of rows) {
    if (row && putUsageRow(row, at)) applied++
  }
  if (applied) {
    bump()
    shared().wbApp?.renderTopbarStatus?.()
  }
  return applied
}

function detailMissing(account: AccountRecord): boolean {
  if (!supportsCreditDetail(account) || account.needsRelogin) return false
  const entry = usageEntryOf(account)
  if (!entry || typeof entry !== 'object' || Array.isArray(entry) || usageFailureOf(entry)) return false
  return !Array.isArray(entry.credits)
}

/** 每个会话只补一次旧版读数的明细；查询失败也必须保留失败状态。 */
async function repairStaleDetail(): Promise<void> {
  if (shapeRepairStarted) return
  shapeRepairStarted = true
  for (const account of allAccounts().filter(detailMissing)) {
    try {
      await queryUsageFor(account.id)
    } catch (error) {
      putUsage(account.id, t('查询失败：{message}', { message: errorMessage(error) }), Date.now())
      bump()
    }
  }
}

export async function syncBalancesSnapshot(): Promise<boolean> {
  try {
    const data = await shared().workbuddyDesktop?.getBalancesSnapshot?.()
    // Token 消耗独立于余额查询变化，不能被余额的时间戳早退挡住。
    const limiterApplied = adoptLimiterSnapshot(data)
    const sortingApplied = !getStore().sortBusy && adoptAccountSorting(data?.sorting)
    const tokenApplied = applyTokenUsage(data) || limiterApplied || sortingApplied
    if (tokenApplied) {
      bump()
      shared().wbApp?.renderTopbarStatus?.()
    }
    const at = Number(data?.at) || 0
    if (!at || at === lastSnapshotAt) return tokenApplied
    lastSnapshotAt = at
    if (!applyBalances(data)) return tokenApplied
    void repairStaleDetail()
    return true
  } catch {
    // 快照读取失败不代表上游余额查询失败，保留当前结果等待下次轮询。
    return false
  }
}

export async function queryUsageFor(id?: string | null): Promise<{ results?: Array<Record<string, unknown>> } | null | undefined> {
  const data = await shared().workbuddyDesktop?.getAllBalances?.(id || undefined)
  await refreshAccountSorting()
  const rows = Array.isArray(data?.results) ? data.results : []
  const at = Date.now()
  const returned = new Set<string>()
  for (const row of rows) {
    if (!row?.id) continue
    const rowId = String(row.id)
    if (id && rowId !== id) continue
    if (row.code === 'refresh_token_expired') void shared().wbApp?.refresh?.()
    returned.add(rowId)
    putUsageRow(row, at)
  }
  if (id) {
    if (!returned.has(id)) putUsage(id, t('未返回余额数据'), at)
    bump()
    return data
  }
  for (const account of batchUsageTargets()) {
    if (!returned.has(account.id)) putUsage(account.id, t('未返回余额数据'), at)
  }
  bump()
  return data
}

export async function queryAllUsage(): Promise<void> {
  if (getStore().usageBusy) return
  const targets = batchUsageTargets()
  if (!targets.length) { toast(t('暂无可查询余额的账号'), 'err'); return }
  patch({ usageBusy: true })
  targets.forEach(account => putUsage(account.id, null, Date.now()))
  bump()
  try {
    const rows = (await queryUsageFor(null))?.results || []
    const ok = rows.filter(row => row.usage && !row.error).length
    const skipped = rows.filter(row => row.code === NOT_CONFIGURED_CODE).length
    const suffix = skipped ? t('（{n} 个未配置查询凭证）', { n: skipped }) : ''
    const failed = rows.length - ok - skipped
    toast(ok + skipped === rows.length
      ? t('✅ 已更新 {n} 个账号的余额{suffix}', { n: ok, suffix })
      : t('已更新 {ok}/{total} 个账号，{failed} 个失败', { ok, total: rows.length, failed }), failed ? 'err' : 'ok')
  } catch (error) {
    const message = errorMessage(error)
    targets.forEach(account => putUsage(account.id, t('查询失败：{message}', { message }), Date.now()))
    bump()
    toast(t('余额查询失败：{message}', { message }), 'err')
  } finally {
    patch({ usageBusy: false })
  }
}

async function runUsageQuery(id: string): Promise<void> {
  if (getStore().usageInflight.has(id)) return
  const inflight = new Set(getStore().usageInflight)
  inflight.add(id)
  putUsage(id, null, Date.now())
  patch({ usageInflight: inflight })
  try {
    await queryUsageFor(id)
  } finally {
    const next = new Set(getStore().usageInflight)
    next.delete(id)
    patch({ usageInflight: next })
  }
}

export async function queryUsageOnce(id: string): Promise<void> {
  if (getStore().usageInflight.has(id)) return
  try {
    await runUsageQuery(id)
    const failure = usageFailureOf(usageEntries().get(id))
    if (failure?.notConfigured) toast(failure.message, 'ok')
    else if (failure) toast(t('余额查询失败：{message}', { message: failure.message }), 'err')
    else toast(t('✅ 已更新余额'))
  } catch (error) {
    putUsage(id, t('查询失败：{message}', { message: errorMessage(error) }), Date.now())
    bump()
    toast(t('余额查询失败：{message}', { message: errorMessage(error) }), 'err')
  }
}

/** 签到或领取套餐后刷新余额，但不能用余额 toast 顶掉刚完成的动作结果。 */
export async function refreshUsageAfterCheckin(id?: string): Promise<void> {
  if (id) {
    const account = findAccount(id)
    if (!account || !supportsUsage(account) || account.needsRelogin) return
    try {
      await runUsageQuery(id)
    } catch (error) {
      putUsage(id, t('查询失败：{message}', { message: errorMessage(error) }), Date.now())
      bump()
    }
    return
  }
  if (getStore().usageBusy) return
  const targets = batchUsageTargets()
  if (!targets.length) return
  patch({ usageBusy: true })
  targets.forEach(account => putUsage(account.id, null, Date.now()))
  bump()
  try {
    await queryUsageFor(null)
  } catch (error) {
    const message = errorMessage(error)
    targets.forEach(account => putUsage(account.id, t('查询失败：{message}', { message }), Date.now()))
    bump()
  } finally {
    patch({ usageBusy: false })
  }
}
