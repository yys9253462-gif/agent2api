type Translate = (key: string, params?: Record<string, string | number>) => string

type CredentialAccount = {
  provider?: string
  hasAccessToken?: boolean
  accessTokenExpiresAt?: number | null
  hasRefreshToken?: boolean
  refreshTokenExpiresAt?: number | null
  refreshExpiresAt?: number | null
  refreshTokenLongLived?: boolean
}

export type CredentialExpiryRow = {
  label: 'Access Token' | 'Refresh Token'
  expiresAt: number | null
  longLived: boolean
}

/**
 * 真正持有**两份各自独立到期**凭证的提供商（白名单）。
 *
 * ── 为什么不能按「有 accessToken 且有 refreshToken」判 ────────
 * OAuth 类提供商本来就同时有这两枚令牌，但那是**一条凭证链**：access token
 * 到期后由 refresh token 续期，用户只关心「还能用多久」这一个读数。按字段
 * 存在性判会让它们全部渲染成两行，把一条凭证链说成两份凭证。
 *
 * ── 判据 ────────────────────────────────────────────────────
 * 只有该家**同时上报两个互不相干的到期时刻**才算，且必须逐个确认后登记：
 *   · codearts —— STS 临时凭据（`accessTokenExpiresAt`）与刷新令牌
 *     （`refreshTokenExpiresAt`，从 refresh JWT 载荷解出）是两条时间线，
 *     续期会换掉整对 AK/SK/STS。
 * 未登记的家一律走单行（只有主凭证那一个读数），即使它们也有 refreshToken。
 */
const DUAL_CREDENTIAL_PROVIDERS: readonly string[] = ['codearts']

export function credentialExpiryMillis(value: unknown): number | null {
  if (typeof value !== 'number' && typeof value !== 'string') return null
  if (typeof value === 'string' && !value.trim()) return null
  const number = Number(value)
  const millis = Number.isFinite(number) ? number < 1e11 ? number * 1000 : number : Date.parse(String(value))
  return Number.isFinite(millis) && millis > 0 ? millis : null
}

export function credentialExpiryRowsOf(account: CredentialAccount, accessExpiry: unknown): CredentialExpiryRow[] {
  // 先按白名单收口：没登记的家即使字段齐全也走单行（见 DUAL_CREDENTIAL_PROVIDERS）
  if (!DUAL_CREDENTIAL_PROVIDERS.includes(String(account.provider ?? ''))) return []
  if (account.hasAccessToken !== true || account.hasRefreshToken !== true) return []
  const refreshExpiry = credentialExpiryMillis(account.refreshTokenExpiresAt ?? account.refreshExpiresAt)
  // 第二行必须真有独立读数：两个到期时刻相同（或都读不到）时，两行说的是同一件事，
  // 只留主凭证那行 —— 否则会出现「Access Token 317 天后 / Refresh Token 317 天后」
  // 这种把一条凭证链拆成两行的画面。
  if (refreshExpiry === null && account.refreshTokenLongLived !== true) return []
  const accessMillis = credentialExpiryMillis(account.accessTokenExpiresAt) ?? credentialExpiryMillis(accessExpiry)
  if (refreshExpiry !== null && accessMillis !== null && refreshExpiry === accessMillis) return []
  return [
    { label: 'Access Token', expiresAt: accessMillis, longLived: false },
    { label: 'Refresh Token', expiresAt: refreshExpiry, longLived: refreshExpiry === null && account.refreshTokenLongLived === true },
  ]
}

export function credentialExpiryText(row: CredentialExpiryRow, t: Translate, now = Date.now()): string {
  if (row.longLived) return t('长期有效')
  if (row.expiresAt === null) return '—'
  const left = row.expiresAt - now
  if (left <= 0) return t('已过期')
  if (left < 3_600_000) return t('{n} 分钟后', { n: Math.max(1, Math.round(left / 60_000)) })
  if (left < 48 * 3_600_000) return t('{n} 小时后', { n: Number((left / 3_600_000).toFixed(1)) })
  return t('{n} 天后', { n: Math.floor(left / 86_400_000) })
}
