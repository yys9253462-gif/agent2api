import { accountReadTools } from './assistant-read-accounts'
import { logReadTools } from './assistant-read-logs'
import { reportReadTools } from './assistant-read-report'
import { diagnosticText } from './assistant-read-data'
import type { AssistantReadApi, AssistantReadTool, ToolArgs, Translate } from './assistant-read-types'

export type { AssistantReadApi, AssistantReadTool, QueryParams } from './assistant-read-types'

export function createAssistantReadTools(api: AssistantReadApi, t: Translate): AssistantReadTool[] {
  return [...accountReadTools(api, t), ...reportReadTools(api, t), ...logReadTools(api, t)].map(tool => ({
    ...tool,
    run: async (args: ToolArgs) => {
      try {
        return await tool.run(args)
      } catch (error) {
        throw new Error(diagnosticText(error instanceof Error ? error.message : String(error), t))
      }
    },
  }))
}

export function assistantReadMeta(t: Translate) {
  return [
    { id: 'list_accounts', label: t('获取账号列表'), desc: t('读取账号、启用状态、优先级与隐藏原因，支持筛选和分页。') },
    { id: 'get_account_detail', label: t('获取账号详情'), desc: t('读取凭证状态、限流、并发、代理和限制器的本地快照。') },
    { id: 'get_routing_queue', label: t('查询路由队列'), desc: t('查看模型候选账号、跳过原因和当前路由策略。') },
    { id: 'get_usage_report', label: t('查询用量报表'), desc: t('查询生产请求的 Token 用量、成功率、排行和趋势。') },
    { id: 'list_request_logs', label: t('查询请求日志'), desc: t('查询请求状态、耗时、账号、错误与重试，不读取对话正文。') },
    { id: 'list_runtime_logs', label: t('查询运行日志'), desc: t('查询凭证、插件和定时任务等运行事件的脱敏摘要。') },
  ]
}
