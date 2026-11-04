import { t } from '../i18n'
import { shared, type PoolItem } from './accounts-shared'

type ProxyPoolData = {
  items: PoolItem[]
}

/** 已缓存的池列表（`null` = 还没读过）；下拉首帧用它避免先空一拍 */
let poolCache: PoolItem[] | null = null
/** 在途读取（并发调用合并成一次） */
let poolInflight: Promise<ProxyPoolData> | null = null
/** 写操作的版本号：`acceptProxyPool` 每写一次 +1（用于「在途读到的旧响应别覆盖新列表」） */
let writeSeq = 0
/** 读取失败的原因（成功后清空）—— 代理列据此显示「读取失败」的说明项 */
let lastPoolError: string | null = null

/** 自愈节流：没有它，「失败 → 重画 → 又拉」会变成死循环 */
let retryAt = 0
let bridgeWarned = false

let lastSignature: string | null = null

function signatureOf(items: PoolItem[]): string {
  return items
    .map(item => [
      item.id,
      item.name || '',
      item.enabled === false ? 0 : 1,
      item.protocol || '',
      item.host || '',
      item.resolved?.protocol || '',
      item.resolved?.host || '',
      item.resolved?.port ?? '',
      item.port ?? '',
      item.resolveError ?? '',
      item.usedBy?.length ?? 0,
      item.usedByServices?.join(',') ?? '',
    ].join(':'))
    .join('|')
}

/** 内容与上次通知过的等价？ */
function unchanged(items: PoolItem[]): boolean {
  return lastSignature !== null && signatureOf(items) === lastSignature
}

function remember(items: PoolItem[]): void {
  lastSignature = signatureOf(items)
}

const listeners = new Set<() => void>()

export function subscribeProxyPool(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

function emit(): void {
  listeners.forEach(listener => listener())
}

/** 已缓存的池列表（`null` = 还没读过） */
export function proxyPoolSnapshot(): PoolItem[] | null {
  return poolCache
}

/** 代理池读取失败的原因（`null` = 没失败） */
export function poolError(): string | null {
  return lastPoolError
}

/** 清掉缓存（下一次调用会真正重新读取） */
export function invalidateProxyPoolCache(): void {
  poolCache = null
  // 签名一起清：清缓存之后界面可能被别的原因重画成「正在读取」，那次重读
  // 即使读到完全相同的内容也必须通知一次，否则会停在「正在读取」上
  lastSignature = null
}

export function acceptProxyPool(items: PoolItem[] | null | undefined): void {
  if (!Array.isArray(items)) return
  poolCache = items
  writeSeq += 1
  lastPoolError = null
  retryAt = 0
  if (unchanged(items)) return
  remember(items)
  emit()
}

/**
 * 读一次代理池（共享缓存；并发调用合并成一次）。
 *
 * 桥异常时可能 resolve 出 `undefined`（而不是
 * reject），那种情况按失败处理 —— 不校验的话会伪装成「池里一条都没有」。
 */
export async function proxyPoolOptions(options: { force?: boolean } = {}): Promise<PoolItem[]> {
  if (!options.force && poolCache) return poolCache
  // 发请求那一刻的写版本号：回来时若已经被写操作推进过，说明手上这份响应更旧
  const seqAtStart = writeSeq
  if (!poolInflight) {
    poolInflight = (async (): Promise<ProxyPoolData> => {
      try {
        const data = await shared().workbuddyDesktop?.getProxyPool?.()
        if (!data || typeof data !== 'object' || !Array.isArray(data.items)) {
          throw new Error(t('代理池响应异常（{type}）', { type: typeof data }))
        }
        // 校验过 items 是数组，这里把它收窄成必填 —— 下游不必再判一次
        return { items: data.items }
      } finally {
        poolInflight = null
      }
    })()
  }
  const data = await poolInflight
  const items = data.items
  // 在途期间可能有写操作已经 accept 了更新的列表（网络代理页刚删了一条）：
  // 那份比这次读到的旧响应新，不能被覆盖回去
  const fresh = writeSeq === seqAtStart || poolCache === null
  lastPoolError = null
  if (!fresh && poolCache) return poolCache
  poolCache = items
  // 真读到了就通知：弹窗里点「重新读取」、切页兜底重读，都会让背后的账号表格
  // 跟着换掉下拉选项（它读的是同步快照，没有通知就看不到新数据）
  if (!unchanged(items)) {
    remember(items)
    emit()
  }
  return poolCache
}

/**
 * 代理列的自愈：没读到就补拉（节流 30 秒）。
 *
 * 成功路径的通知在 `proxyPoolOptions` 里（真读到就 emit）；这里只管失败 ——
 * 失败不写缓存，得把原因通知出去，代理列才能显示「读取失败（重试中）」。
 */
export function ensureProxyPoolOptions(): void {
  if (poolCache || Date.now() < retryAt) return
  if (typeof shared().workbuddyDesktop?.getProxyPool !== 'function') {
    // 桥没挂上（加载失败等）：本页面会话内不再尝试，并告警一次
    if (!bridgeWarned) {
      bridgeWarned = true
      console.warn('[accounts] 桥未提供 getProxyPool：代理列的「已保存的代理」选项不可用')
    }
    retryAt = Infinity
    return
  }
  retryAt = Date.now() + 30_000
  void proxyPoolOptions()
    .then(() => { retryAt = 0 })
    .catch(error => {
      lastPoolError = error instanceof Error ? error.message : String(error)
      emit()
    })
}

/**
 * 强制重读一次（切回账号页时调，见 accounts-data 的 refreshProxyPool）。
 *
 * 与 `ensureProxyPoolOptions` 的区别：这个**一定**发请求（清节流），用来覆盖
 * 「池在应用外被改过」或「网络代理页这次没加载过」的情况。
 *
 * **不先清缓存**：清了会让界面在请求在途的那一瞬退回「正在读取代理列表…」，
 * 而请求回来内容又没变时（最常见的情况）不会通知 —— 那一格就停在「正在读取」
 * 上了。留着旧缓存，读到新内容才替换、才通知；失败时旧列表还在，比空着有用。
 */
export async function refreshProxyPool(): Promise<void> {
  retryAt = 0
  try {
    await proxyPoolOptions({ force: true })
  } catch (error) {
    lastPoolError = error instanceof Error ? error.message : String(error)
    emit()
  }
}
