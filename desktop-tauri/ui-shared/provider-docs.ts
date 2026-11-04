export const PROVIDER_CAPABILITY_KEYS = [
  'chat', 'refresh', 'model_refresh', 'usage', 'checkin', 'claims',
] as const

export type ProviderCapabilityKey = typeof PROVIDER_CAPABILITY_KEYS[number]
export type ProviderCapabilityStatus = 'supported' | 'implemented_unverified' | 'unsupported' | 'not_applicable' | 'undeclared'
export type PluginDocumentText = string | Record<string, string>

export type ProviderCapabilityCell = {
  status: ProviderCapabilityStatus
  note?: PluginDocumentText | null
}

export type ProviderDocumentation = {
  id: string
  label: string
  enabled: boolean
  loaded: boolean
  capabilities: Record<ProviderCapabilityKey, ProviderCapabilityCell>
  notes: PluginDocumentText[]
}

export type PluginProviderDocumentation = {
  id: string
  name: string
  version: string
  description?: PluginDocumentText | null
  providers: ProviderDocumentation[]
}

export type ProviderDocsPayload = {
  providerDocs?: PluginProviderDocumentation[]
}

export function documentText(text: PluginDocumentText | null | undefined, locale: string): string {
  if (typeof text === 'string') return text
  if (!text || typeof text !== 'object') return ''
  const aliases: Record<string, string> = { 'zh-Hans': 'zh-CN', 'zh-Hant': 'zh-TW' }
  const candidates = [locale, aliases[locale], locale.split('-')[0], 'zh-CN', 'zh-Hans', 'en']
  for (const key of candidates) {
    const value = key ? text[key] : undefined
    if (typeof value === 'string' && value.trim()) return value
  }
  return Object.values(text).find(value => typeof value === 'string' && value.trim()) || ''
}
