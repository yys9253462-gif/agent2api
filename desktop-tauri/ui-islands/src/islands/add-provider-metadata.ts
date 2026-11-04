import { shared, type CustomProviderRecord, type PresetRecord } from './add-account-bridge'

export function presetOfProvider(provider: CustomProviderRecord | null | undefined): PresetRecord | null {
  const key = provider?.presetKey || ''
  return key ? shared().wbPresetProviders?.presetOf?.(key) || null : null
}

export function providerPresetIcon(provider: CustomProviderRecord | null | undefined): string {
  const key = provider?.presetKey || ''
  return key ? shared().wbPresetProviders?.iconOf?.(key) || '' : ''
}

export function providerPresetIconOf(id: string): string {
  const provider = (shared().wbProviders?.customList?.() || []).find(item => item.id === id)
  return providerPresetIcon(provider)
}

export function providerSiteLabel(baseUrl: string | undefined): string {
  if (!baseUrl) return ''
  try {
    const url = new URL(baseUrl)
    return url.host + url.pathname.replace(/\/+$/, '')
  } catch {
    return baseUrl
  }
}

export function sameProviderEndpoint(
  provider: CustomProviderRecord,
  protocol: string,
  baseUrl: string,
): boolean {
  if (provider.protocol !== protocol) return false
  const normalize = (value: string): string => new URL(value.trim()).href.replace(/\/+$/, '')
  try {
    return normalize(provider.baseUrl || '') === normalize(baseUrl)
  } catch {
    return false
  }
}
