import { apiCall } from '../api/client'
import { fetchAccounts, fetchUsageSnapshot } from '../api/gateway'
import { t } from '../i18n'
import { createAssistantReadTools } from '../../../ui-shared/assistant-read-tools'
import type { AssistantReadApi, QueryParams } from '../../../ui-shared/assistant-read-types'

function queryString(query: QueryParams): string {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) params.set(key, String(value))
  return `?${params.toString()}`
}

const api: AssistantReadApi = {
  accounts: () => fetchAccounts(),
  usageSnapshot: () => fetchUsageSnapshot(),
  connections: () => apiCall('/api/accounts/connections'),
  routingQueue: model => apiCall('/api/routing/queue' + (model ? queryString({ model }) : '')),
  usageReport: (range, dates, options) => apiCall('/api/stats/summary' + queryString({
    range, ...(dates ? { startDate: dates.start, endDate: dates.end } : {}),
    ...(options?.apiKey ? { apiKey: options.apiKey } : {}),
  })),
  requestLogs: query => apiCall('/api/stats/requests' + queryString(query)),
  runtimeLogs: query => apiCall('/api/logs' + queryString(query)),
}

export const ASSISTANT_READ_TOOLS = createAssistantReadTools(api, t)
