import { t } from '../i18n'
import * as customSource from './models-custom-source'
import type { ManageModel, ManageView } from './models-custom-source'
import { capabilitiesOf, reasoningLevelsOf } from './model-capability'
import type { AgentModel } from './agent-types'

export type CatalogModel = AgentModel & { label: string; provider: string; providers: string[] }

type CatalogWindow = { workbuddyDesktop?: { getModelManage?: () => Promise<ManageView> } }

export async function loadCatalog(): Promise<CatalogModel[]> {
  const call = (window as unknown as CatalogWindow).workbuddyDesktop?.getModelManage
  if (!call) throw new Error(t('当前环境无法读取网关模型目录'))
  const view = await call()
  const models: ManageModel[] = [...(view.models || [])]
  const custom = customSource.list().flatMap(provider => provider.id ? [customSource.buildView(provider.id)].filter((value): value is NonNullable<typeof value> => value !== null) : [])
  models.push(...custom.flatMap(view => view.models))
  const mappings = [...(view.mappings || []), ...custom.flatMap(view => view.mappings)]
  const result = new Map<string, CatalogModel>()
  for (const row of models) {
    if (!row.enabled || row.kind === 'image') continue
    const capabilities = capabilitiesOf(row)
    const reasoning = reasoningLevelsOf(row)
    const model: CatalogModel = {
      id: row.id, name: row.name || row.id, provider: row.provider, providers:[row.provider], label: `${row.name || row.id} · ${row.providerLabel || row.provider}`,
      contextWindow: typeof capabilities.maxInputTokens === 'number' ? capabilities.maxInputTokens : null,
      maxOutputTokens: typeof capabilities.maxOutputTokens === 'number' ? capabilities.maxOutputTokens : null,
      supportsTools: typeof capabilities.supportsToolCall === 'boolean' ? capabilities.supportsToolCall : null,
      supportsImages: typeof capabilities.supportsImages === 'boolean' ? capabilities.supportsImages : null,
      supportsReasoning: typeof capabilities.supportsReasoning === 'boolean' ? capabilities.supportsReasoning : null,
      reasoningLevels: reasoning?.levels || [], effort: '',
    }
    const add = (model: CatalogModel) => {
      const existing = result.get(model.id)
      if (!existing) result.set(model.id,model)
      else if (!existing.providers.includes(row.provider)) existing.providers.push(row.provider)
    }
    add(model)
    for (const mapping of mappings.filter(mapping => mapping.enabled && mapping.target === row.id && mapping.provider === row.provider)) {
      add({ ...model, providers:[row.provider], id: mapping.alias, name: mapping.alias, label: `${mapping.alias} · ${row.providerLabel || row.provider}`, effort: mapping.reasoning || '' })
    }
  }
  return [...result.values()]
}
