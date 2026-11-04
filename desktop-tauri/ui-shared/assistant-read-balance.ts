import type { ToolArgs, Translate } from './assistant-read-types'
import { accountName, accountsOf, booleanArg, diagnosticText, expectObject, expectRows, fields, findAccount, number, object, page, pageArgs, PAGE_PARAMS, result, stringArg } from './assistant-read-data'
import { usageSummary } from './assistant-read-accounts'
import type { ImageToolContext } from './assistant-image-tools'
import { checkToolSignal } from './assistant-tool-execution'

type BalanceApi = {
  accounts: () => Promise<unknown>
  usageSnapshot: () => Promise<unknown>
  refreshBalance: (id: string) => Promise<unknown>
}

export function createBalanceTool(api: BalanceApi, t: Translate): {
  id: string; label: string; kind: 'read'; desc: string; params: Record<string, unknown>
  run: (args: ToolArgs, context?: ImageToolContext) => Promise<{ text: string; summary: string }>
} {
  return {
    id: 'list_account_balance', label: t('查询账号余额'), kind: 'read',
    desc: '分页读取账号最近一次缓存的余额和额度，不把未查询当作零。默认只读快照；refresh=true 只实时刷新本页账号，会请求上游并可能触发已配置的余额不足自动禁用规则，需要按权限模式确认。每次返回 total/hasMore/nextOffset，hasMore=true 时继续翻页；翻页默认不刷新。',
    params: {
      type: 'object', additionalProperties: false,
      properties: {
        account: { type: 'string', description: '账号 ID 或唯一的展示名；不填为全部' },
        provider: { type: 'string', description: '提供商 ID，用于筛选和区分同名账号' },
        refresh: { type: 'boolean', description: 'true 实时刷新本页；默认 false，只读取缓存' },
        ...PAGE_PARAMS,
      },
    },
    run: async (args, context) => {
      checkToolSignal(context)
      const ref = stringArg(args, 'account', t)
      const provider = stringArg(args, 'provider', t)
      const refresh = booleanArg(args, 'refresh', t) === true
      const { offset, limit } = pageArgs(args, t)
      const { accounts } = accountsOf(await api.accounts(), t)
      const selected = ref ? [findAccount(accounts, ref, provider, t)]
        : accounts.filter(account => !provider || String(account.provider).toLowerCase() === provider.toLowerCase())
      const selectedPage = page(selected.slice(offset).map(account => fields(account, ['id'])), offset, limit, selected.length, 3000)
      if (refresh) {
        for (const item of selectedPage.items) {
          checkToolSignal(context)
          await api.refreshBalance(String(item.id))
        }
      }
      const snapshot = expectObject(await api.usageSnapshot(), 'usageSnapshot', t)
      const cache = expectRows(snapshot, 'results', 'usageSnapshot.results', t)
      const entries = selected.slice(offset, offset + selectedPage.returned).map(account => {
        const row = cache.find(row => row.id === account.id)
        return {
          ...fields(account, ['id', 'provider', 'enabled']), name: accountName(account),
          cached: !refresh, queried: Boolean(row), at: row ? number(row.at) : null,
          ...fields(row, ['code']), error: diagnosticText(row?.error, t),
          usage: row?.usage ? usageSummary(object(row.usage), t) : null,
          lastSuccessAt: row ? number(row.lastSuccessAt) : null,
          lastUsage: row?.lastUsage ? usageSummary(object(row.lastUsage), t) : null,
        }
      })
      return result({ asOf: Date.now(), accounts: page(entries, offset, limit, selected.length), refreshed: refresh }, t('{n} 个账号', { n: selected.length }))
    },
  }
}
