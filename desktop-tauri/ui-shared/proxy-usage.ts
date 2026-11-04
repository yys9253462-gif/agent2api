export type ProxyService = 'update'

export type ProxyUsage = {
  usedBy?: Array<{ id?: string; name?: string; enabled?: boolean }>
  usedByServices?: ProxyService[]
}

export function hasProxyUsage(item: ProxyUsage): boolean {
  return Boolean(item.usedBy?.length || item.usedByServices?.length)
}

export function proxyServiceLabels(item: ProxyUsage, t: (key: string) => string): string[] {
  return [...new Set(item.usedByServices || [])]
    .filter(service => service === 'update')
    .map(() => t('软件 / 插件更新'))
}
