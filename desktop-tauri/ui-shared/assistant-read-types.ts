export type ToolResult = { text: string; summary: string }
export type ToolArgs = Record<string, unknown>
export type QueryParams = Record<string, string | number | boolean>
export type Translate = (key: string, params?: Record<string, string | number>) => string

export type AssistantReadTool = {
  id: string
  label: string
  kind: 'read'
  desc: string
  params: Record<string, unknown>
  run: (args: ToolArgs) => Promise<ToolResult>
}

export type AssistantReadApi = {
  accounts: () => Promise<unknown>
  usageSnapshot: () => Promise<unknown>
  connections: () => Promise<unknown>
  routingQueue: (model?: string) => Promise<unknown>
  usageReport: (range: string, dates?: { start: string; end: string }, options?: { apiKey?: string }) => Promise<unknown>
  requestLogs: (query: QueryParams) => Promise<unknown>
  runtimeLogs: (query: QueryParams) => Promise<unknown>
}
