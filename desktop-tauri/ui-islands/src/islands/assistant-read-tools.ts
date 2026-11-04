import { t } from '../i18n'
import { assistantReadMeta, createAssistantReadTools } from '../../../ui-shared/assistant-read-tools'
import type { AssistantReadApi, QueryParams } from '../../../ui-shared/assistant-read-types'

type ReadBridge = {
  getAccounts?: () => Promise<unknown>
  getBalancesSnapshot?: () => Promise<unknown>
  getAccountConnections?: () => Promise<unknown>
  getRoutingQueue?: (model?: string) => Promise<unknown>
  getStatsSummary?: (range: string, dates?: { start: string; end: string }, options?: { apiKey?: string }) => Promise<unknown>
  getStatsRequests?: (query: QueryParams | string) => Promise<unknown>
  getLogs?: (query: QueryParams) => Promise<unknown>
}

function bridge(): ReadBridge {
  const api = (window as unknown as { workbuddyDesktop?: ReadBridge }).workbuddyDesktop
  if (!api) throw new Error(t('桌面桥不可用（本页需要在 Agent2API 客户端里打开）'))
  return api
}

function method<K extends keyof ReadBridge>(key: K): NonNullable<ReadBridge[K]> {
  const value = bridge()[key]
  if (typeof value !== 'function') throw new Error(t('当前客户端缺少读取接口：{name}', { name: key }))
  return value as NonNullable<ReadBridge[K]>
}

const api: AssistantReadApi = {
  accounts: () => method('getAccounts')(),
  usageSnapshot: () => method('getBalancesSnapshot')(),
  connections: () => method('getAccountConnections')(),
  routingQueue: model => method('getRoutingQueue')(model),
  usageReport: (range, dates, options) => method('getStatsSummary')(range, dates, options),
  requestLogs: query => {
    const params = new URLSearchParams()
    for (const [key, value] of Object.entries(query)) params.set(key, String(value))
    return method('getStatsRequests')(params.toString())
  },
  runtimeLogs: query => method('getLogs')(query),
}

export const ASSISTANT_READ_TOOLS = createAssistantReadTools(api, t)
export const ASSISTANT_READ_META = assistantReadMeta(t)
