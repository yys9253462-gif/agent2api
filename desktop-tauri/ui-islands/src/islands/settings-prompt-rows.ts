import type { PromptState, ProviderPromptState } from './settings-state'

export function providerPromptRows(prompt: PromptState): ProviderPromptState[] {
  const configured = new Map(prompt.providers.map(item => [item.id, item]))
  const ids = new Set([
    ...configured.keys(),
    ...Object.keys(prompt.gateway),
    ...Object.keys(prompt.gatewayText),
  ])
  const ordered = [
    ...prompt.options.map(item => item.id).filter(id => ids.has(id)),
    ...ids,
  ]
  const seen = new Set<string>()
  const rows: ProviderPromptState[] = []
  for (const id of ordered) {
    if (seen.has(id)) continue
    seen.add(id)
    const item = configured.get(id)
    rows.push(item ? {
      ...item,
      gateway: prompt.gateway[id] ?? true,
      configured: true,
    } : {
      id,
      mode: prompt.mode,
      file: prompt.file,
      text: prompt.text,
      source: prompt.source,
      lines: prompt.lines,
      fileError: prompt.fileError,
      gateway: prompt.gateway[id] ?? true,
      configured: false,
    })
  }
  return rows
}
