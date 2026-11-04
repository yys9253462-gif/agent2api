export type AccountSortRule =
  | { key: 'credit'; on: boolean; dir: 'soon' | 'late' }
  | { key: 'added'; on: boolean; dir: 'new' | 'old' }

export type AccountSorting = {
  updatedAt: number
  configured: Record<string, AccountSortRule[]>
  rules: Record<string, AccountSortRule[]>
  providerOrder: string[]
  accountOrder: Record<string, string[]>
  creditExpiresAt: Record<string, number | null>
}
