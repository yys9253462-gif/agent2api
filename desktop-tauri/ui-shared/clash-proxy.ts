export type ClashProxySnapshot = {
  available?: boolean
  error?: string | null
  dir?: string | null
  options?: Array<{
    uid?: string
    name?: string
    port?: number
    kind?: 'mixed' | 'listener'
    enabled?: boolean
    profileActive?: boolean
  }>
}
