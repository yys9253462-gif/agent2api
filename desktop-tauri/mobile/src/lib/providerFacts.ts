/**
 * 按提供商区分的事实（有效期字段 / 标识字段 / 长期有效 / 余额能力）。
 *
 * ── 事实来源：插件申报，不是客户端查表 ──────────────────────
 * 「有效期落在哪个键」按家不同：WorkBuddy 写 `expiresAt`，小浣熊写
 * `tokenExpiresAt`，OrcaTerm 写 `tokenExpireAt`（少一个 s），KukuAI 压根没有
 * 这个字段。这些**全是上游事实**，原先在桌面端与手机端各存一份手写表 ——
 * 手机端那份还是从桌面端摘抄的（见本文件的历史注释），摘错一次就是「有效期
 * 一直显示 —」。
 *
 * 现在的事实来源是**插件在握手时申报的 `AccountFeatures`**，由后端挂在
 * `/api/session` 的 `accountFeatures` 上一起下发。手机端只需要「收下 + 查」，
 * 新插件装上就自动正确，不必再改客户端。
 *
 * ── 兜底 ────────────────────────────────────────────────────
 * 旧插件不申报（`accountFeatures` 里没有这一家）时走 `FALLBACK`：
 * 两个键都看、取先有值的那个。宁可兜底，也不要因为漏登记而少一行读数。
 */

/** 一个提供商的账号行参数（与协议里 `AccountFeatures` 同形，只取手机端用到的） */
export type ProviderFacts = {
  /** 有效期字段名的**候选列表**（按顺序取第一个能解析出正数的）；空 = 没有这个概念 */
  expiry: readonly string[]
  /** 账号标识字段名的候选列表 */
  identifier: readonly string[]
  /** 主凭证长期有效（没有固定到期时刻） */
  longLived: boolean
  /** 这家有没有余额查询能力 */
  usage: boolean
}

/**
 * 插件申报表（`provider id → 申报`），由 `/api/session` 的 `accountFeatures` 带回。
 *
 * 模块级缓存而不是 React state：读它的地方都在渲染函数里（列表行、详情页），
 * 而它每次会话刷新才变一次 —— 换一份新 Map 就够。
 */
let declared: ReadonlyMap<string, ProviderFacts> = new Map()
/** 上一次归一用的原始对象（引用比较：`/api/session` 换一份才重归一） */
let declaredSource: unknown = null

/**
 * 收下后端带回的申报表。
 *
 * 形状是协议里 `AccountFeatures` 的 serde 输出（snake_case）。字段缺省按
 * `FALLBACK` 填 —— 这样消费侧不必到处判「键在不在」。
 */
export function adoptProviderFacts(raw: unknown): void {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    declared = new Map()
    return
  }
  const next = new Map<string, ProviderFacts>()
  for (const [id, value] of Object.entries(raw as Record<string, unknown>)) {
    if (!id || !value || typeof value !== 'object' || Array.isArray(value)) continue
    next.set(id, normalize(value as Record<string, unknown>))
  }
  declared = next
}

/** 把协议形态归一成手机端形态。 */
function normalize(raw: Record<string, unknown>): ProviderFacts {
  return {
    expiry: keyList(raw.expiry),
    identifier: keyList(raw.identifier, FALLBACK.identifier),
    longLived: raw.long_lived === true,
    usage: raw.usage === true,
  }
}

/** 键名候选列表：数组原样取字符串项，单个字符串包成一项，其余走兜底。 */
function keyList(value: unknown, fallback: readonly string[] = []): readonly string[] {
  if (typeof value === 'string') return value ? [value] : []
  if (!Array.isArray(value)) return fallback
  return value.filter((item): item is string => typeof item === 'string' && item.length > 0)
}

const FALLBACK: ProviderFacts = {
  expiry: ['expiresAt', 'tokenExpiresAt'],
  identifier: ['userId'],
  longLived: false,
  usage: false,
}

/**
 * 某个提供商的账号行参数。查询顺序：**插件申报 → 通用兜底**。
 *
 * `session` 参数可选：调用方手里有会话快照时传进来，函数会按引用比较决定
 * 要不要重新归一（`/api/session` 每次刷新都换一份新对象）。
 */
export function providerFacts(
  id: string | undefined,
  session?: { accountFeatures?: Record<string, unknown> } | null,
): ProviderFacts {
  syncDeclared(session)
  return (id && declared.get(id)) || FALLBACK
}

/** 会话快照变了才重新归一（引用比较；没有会话时保持现状） */
function syncDeclared(session?: { accountFeatures?: Record<string, unknown> } | null): void {
  if (!session) return
  const raw = session.accountFeatures
  if (raw === declaredSource) return
  declaredSource = raw
  adoptProviderFacts(raw)
}

/** 当前申报表（排障用；界面请走 `providerFacts`） */
export function declaredProviderFacts(): ReadonlyMap<string, ProviderFacts> {
  return declared
}

/**
 * 这家账号能不能查余额：自定义提供商看 balanceQuery.scheme，插件家看申报。
 *
 * 访客账号（`isGuest`）一律 false：智谱清言的访客没有积分体系（上游接口
 * 一律 401），查询只会得到恒定的失败。判据与桌面端 `accounts-domain` 的同名
 * 函数、以及后端 `usage_query::supports_usage` 同口径 —— 三处必须一致，
 * 否则界面算的目标集合与后端返回的行会对不上。
 */
export function supportsUsage(account: {
  provider?: string
  balanceQuery?: unknown
  isGuest?: boolean
}): boolean {
  if (account.isGuest === true) return false
  const id = String(account.provider || '')
  if (id.startsWith('custom-')) {
    const scheme = (account.balanceQuery as { scheme?: unknown } | undefined)?.scheme
    return Boolean(scheme)
  }
  return providerFacts(id).usage
}

/** 把值归一到毫秒（秒与毫秒在库里都出现过，见桌面端 accounts-domain 的同名函数） */
function expiryMillis(value: unknown): number {
  const raw = Number(value) || 0
  if (raw <= 0) return 0
  return raw < 1e11 ? raw * 1000 : raw
}

/**
 * 取账号的有效期时间戳（毫秒，0 = 记录里没有）。
 *
 * 按申报的候选键**顺序取第一个能解析出正数**的：插件新落的记录与存量旧投影的
 * 键名不同（小浣熊是 `expiresAt` / `tokenExpiresAt`），两套都列在申报里。
 * 判据是「能解析出正数」而不是「有值」—— 一个 `null` 不该把后面有值的键挡掉。
 */
export function expiryOf(account: {
  provider?: string
  expiresAt?: number
  tokenExpiresAt?: number
  [key: string]: unknown
}, session?: { accountFeatures?: Record<string, unknown> } | null): number {
  const facts = providerFacts(account.provider, session)
  for (const key of facts.expiry) {
    const value = expiryMillis(account[key])
    if (value > 0) return value
  }
  return 0
}

/** 这家有没有「有效期」这个概念（决定详情里渲不渲染那一格） */
export function hasExpiry(provider: string | undefined): boolean {
  const facts = providerFacts(provider)
  return facts.expiry.length > 0 || facts.longLived
}

/**
 * 这家的主凭证是不是**长期有效**（没有固定到期时刻）。
 *
 * 有它的家照样渲染「有效期」那一格，只是值写「长期有效」而不是「—」：
 * 「—」读作「记录里没有到期时间」，长期有效读作「本来就不会到期」。
 */
export function isLongLivedCredential(provider: string | undefined): boolean {
  return providerFacts(provider).longLived
}
