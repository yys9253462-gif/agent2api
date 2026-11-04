import type { AssistantReadApi, AssistantReadTool, Translate } from './assistant-read-types'
import {
  accountMeta, accountsOf, diagnosticText, enumArg, expectObject, expectRows, fields, number, object, page, pageArgs,
  PAGE_PARAMS, result, rows, stringArg,
} from './assistant-read-data'

function routeEntry(value: Record<string, unknown>, t: Translate) {
  return {
    ...fields(value, ['id', 'name', 'provider', 'providerLabel', 'state', 'resetAt', 'priority']),
    reason: diagnosticText(value.reason, t),
  }
}

function dimension(value: Record<string, unknown>) {
  return fields(value, ['id', 'model', 'label', 'requests', 'success', 'failures', 'totalTokens', 'tokens', 'percentage',
    'successRate', 'promptTokens', 'completionTokens', 'cacheReadTokens', 'cacheRate', 'lastUsedAt',
    'tokenDetailsComplete', 'source', 'filterValue', 'state'])
}

function dateArg(args: Record<string, unknown>, key: string, t: Translate): string {
  const value = stringArg(args, key, t, 10)
  const date = new Date(`${value}T00:00:00Z`)
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || !Number.isFinite(date.getTime()) || date.toISOString().slice(0, 10) !== value) {
    throw new Error(t('参数 {name} 必须是有效日期（YYYY-MM-DD）', { name: key }))
  }
  return value
}

export function reportReadTools(api: AssistantReadApi, t: Translate): AssistantReadTool[] {
  const getRoutingQueue: AssistantReadTool = {
    id: 'get_routing_queue', label: t('查询路由队列'), kind: 'read',
    desc: '读取指定模型（对外模型名/映射别名）在当前策略下的候选账号、被跳过账号及原因。回答「为什么不用某个账号」「还有哪些备用线路」「这个模型能走谁」前调用。model 不填时读取不限模型的全局队列。结果是静态预演，不会消耗轮转游标或改变会话粘性；真实请求还受轮转/粘性和状态变化影响，pickedId 不保证就是下一次真实请求的账号。ordered 是可尝试候选，blocked 是被跳过账号；modelKnown=false 表示模型不在当前路由目录中，不能把返回内容当成该模型的可用候选。分页 hasMore 时按 nextOffset 继续取。',
    params: {
      type: 'object', additionalProperties: false,
      properties: {
        model: { type: 'string', description: '对外模型名或映射别名；不填 = 全局队列' },
        section: { type: 'string', enum: ['ordered', 'blocked', 'both'], description: 'ordered = 候选，blocked = 被跳过，both = 两段（默认）；各段独立分页' },
        ...PAGE_PARAMS,
      },
    },
    run: async args => {
      const model = stringArg(args, 'model', t, 128)
      const section = enumArg(args, 'section', ['ordered', 'blocked', 'both'], 'both', t)
      const { offset, limit } = pageArgs(args, t)
      const [queue, accountData] = await Promise.all([api.routingQueue(model || undefined), api.accounts()])
      const data = expectObject(queue, 'routingQueue', t)
      const { snapshot, accounts } = accountsOf(accountData, t)
      const visible = new Set(accounts.map(account => String(account.id)))
      const ordered = expectRows(data, 'ordered', 'routingQueue.ordered', t).filter(row => visible.has(String(row.id)))
      const blocked = expectRows(data, 'blocked', 'routingQueue.blocked', t).filter(row => visible.has(String(row.id)))
      const payload: Record<string, unknown> = {
        asOf: Date.now(), preview: true,
        ...fields(data, ['strategy', 'sticky', 'model', 'modelKnown', 'lastRequestModel']),
        pickedId: fields(ordered[0], ['id']).id ?? '', hiddenAccounts: accountMeta(snapshot).hiddenAccounts,
      }
      if (section !== 'blocked') payload.ordered = page(ordered.slice(offset).map(row => routeEntry(row, t)), offset, limit, ordered.length, 9000)
      if (section !== 'ordered') payload.blocked = page(blocked.slice(offset).map(row => routeEntry(row, t)), offset, limit, blocked.length, 9000)
      return result(payload, t('{n} 个候选，{m} 个跳过', { n: ordered.length, m: blocked.length }))
    },
  }

  const getUsageReport: AssistantReadTool = {
    id: 'get_usage_report', label: t('查询用量报表'), kind: 'read',
    desc: '读取本地生产请求报表，回答「今天用了多少 Token」「这周哪个账号消耗最多」「成功率怎样」「用量趋势」之前调用。range 的日期边界按网关所在机器的本地自然日，自定义日期含首尾两天。助手对话和模型测试按后端口径排除，不是所有上游消费的总账；余额请用 list_account_balance。summary 默认给概览和各维度前 10 项（含分页元信息），要完整排行请选 providers/accounts/models/clients/apiKeys 并分页；apiKeyDetail 按 filterValue 查询该 Key 的客户端和模型分布，不返回密钥值；dailyTrend/cacheTrend24h 可按需查看趋势。successRate 是 0~1 比例，模型统计按实际上游模型名（旧行回落请求模型），不是映射别名。',
    params: {
      type: 'object', additionalProperties: false,
      properties: {
        range: { type: 'string', enum: ['today', 'yesterday', '7', '30', 'month', 'all', 'custom'], description: '默认 today；custom 时必须带 startDate/endDate' },
        startDate: { type: 'string', description: '自定义范围开始日期，YYYY-MM-DD（包含）' },
        endDate: { type: 'string', description: '自定义范围结束日期，YYYY-MM-DD（包含）' },
        section: { type: 'string', enum: ['summary', 'providers', 'accounts', 'models', 'clients', 'apiKeys', 'apiKeyDetail', 'dailyTrend', 'cacheTrend24h'], description: '默认 summary；其它段可分页完整读取' },
        apiKey: { type: 'string', description: 'apiKeyDetail 必填：使用 apiKeys 行中的 filterValue（key:<id>/env/none/unknown），不传密钥值' },
        ...PAGE_PARAMS,
      },
    },
    run: async args => {
      const range = enumArg(args, 'range', ['today', 'yesterday', '7', '30', 'month', 'all', 'custom'], 'today', t)
      const section = enumArg(args, 'section', ['summary', 'providers', 'accounts', 'models', 'clients', 'apiKeys', 'apiKeyDetail', 'dailyTrend', 'cacheTrend24h'], 'summary', t)
      const { offset, limit } = pageArgs(args, t)
      const dates = range === 'custom' ? { start: dateArg(args, 'startDate', t), end: dateArg(args, 'endDate', t) } : undefined
      if (dates && dates.start > dates.end) throw new Error(t('开始日期不能晚于结束日期'))
      const apiKey = stringArg(args, 'apiKey', t, 128)
      if (section === 'apiKeyDetail' && !apiKey) throw new Error(t('查看 API Key 详情必须指定 API Key 身份'))
      const data = expectObject(await api.usageReport(range, dates, apiKey ? { apiKey } : undefined), 'usageReport', t)
      const overview = expectObject(data.overview, 'usageReport.overview', t)
      if (number(overview.requests) === null || number(overview.successful) === null || number(overview.tokens) === null) {
        throw new Error(t('读取响应不完整：{name}', { name: 'usageReport.overview' }))
      }
      const requests = number(overview.requests) as number
      const successful = number(overview.successful) as number
      const payload: Record<string, unknown> = {
        asOf: Date.now(), excludesAssistantAndTests: true, calendar: 'gateway-local',
        ...fields(data, ['range', 'startDate', 'endDate']),
        overview: {
          ...fields(overview, ['requests', 'successful', 'tokens', 'activeDays', 'streak']),
          successRate: requests ? successful / requests : 0,
          topModel: overview.topModel ? dimension(object(overview.topModel)) : null,
        },
      }
      if (section === 'summary') {
        for (const key of ['providers', 'accounts', 'models', 'clients', 'apiKeys']) {
          const items = expectRows(data, key, `usageReport.${key}`, t)
          payload[key] = page(items.map(dimension), 0, 10, items.length, 3000)
        }
        const rates: Record<string, unknown> = {}
        const source = expectObject(data.cacheRates, 'usageReport.cacheRates', t)
        for (const key of ['last10m', 'last1h', 'last24h', 'last7d']) {
          if (source[key] !== undefined) rates[key] = fields(source[key], ['hitTokens', 'inputTokens', 'rate'])
        }
        payload.cacheRates = rates
      } else if (section === 'apiKeyDetail') {
        const detail = expectObject(data.apiKeyDetail, 'usageReport.apiKeyDetail', t)
        payload.apiKeyDetail = { ...fields(detail, ['filterValue', 'label', 'requests', 'totalTokens', 'detailsComplete']) }
        for (const key of ['clients', 'models']) {
          const items = expectRows(detail, key, `usageReport.apiKeyDetail.${key}`, t)
          object(payload.apiKeyDetail)[key] = page(items.slice(offset).map(dimension), offset, limit, items.length)
        }
      } else {
        const items = expectRows(data, section, `usageReport.${section}`, t)
        const projected = items.slice(offset).map(item => {
          if (section !== 'dailyTrend' && section !== 'cacheTrend24h') return dimension(item)
          const models = rows(item.models)
          return {
            ...fields(item, ['date', 'hour', 'tokens', 'requests', 'totalTokens', 'hitTokens', 'inputTokens', 'rate']),
            models: page(models.map(dimension), 0, 10, models.length, 2000),
            ...(section === 'dailyTrend' ? {
              clients: page(rows(item.clients).map(dimension), 0, 10, rows(item.clients).length, 2000),
              apiKeys: page(rows(item.apiKeys).map(dimension), 0, 10, rows(item.apiKeys).length, 2000),
            } : {}),
          }
        })
        payload[section] = page(projected, offset, limit, items.length)
      }
      return result(payload, t('{requests} 次请求，{tokens} Token', { requests, tokens: number(overview.tokens) as number }))
    },
  }

  return [getRoutingQueue, getUsageReport]
}
