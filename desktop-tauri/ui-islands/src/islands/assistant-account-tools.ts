import { t } from '../i18n'
import type { AssistantTool } from './assistant-tools'
import { accountName, accountsOf, findAccount as resolveAccount } from '../../../ui-shared/assistant-read-data'
import { checkToolSignal, providerGroups, runToolBatch } from '../../../ui-shared/assistant-tool-execution'

type AccountRow = Record<string, unknown>
type AccountBridge = {
  getAccounts: () => Promise<unknown>
  updateAccount: (id: string, patch: Record<string, unknown>) => Promise<unknown>
  reorderAccounts: (queue: string[]) => Promise<unknown>
}

function bridge(): AccountBridge {
  const api = (window as unknown as { workbuddyDesktop?: AccountBridge }).workbuddyDesktop
  if (!api) throw new Error(t('桌面桥不可用（本页需要在 Agent2API 客户端里打开）'))
  return api
}

function str(value: unknown, max = 256): string {
  return typeof value === 'string' ? value.trim().slice(0, max) : ''
}
function num(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}
function list(value: unknown): AccountRow[] {
  return Array.isArray(value) ? value.filter((item): item is AccountRow => Boolean(item) && typeof item === 'object' && !Array.isArray(item)) : []
}
function refreshApp(): void {
  const app = (window as unknown as { wbApp?: { refresh?: () => void } }).wbApp
  void app?.refresh?.()
}

export async function readAccounts(): Promise<AccountRow[]> {
  const read = bridge().getAccounts
  if (typeof read !== 'function') throw new Error(t('当前客户端缺少读取接口：{name}', { name: 'getAccounts' }))
  return accountsOf(await read(), t).accounts
}

export function accountLabel(account: AccountRow): string {
  return accountName(account) || t('未命名账号')
}

export function findAccount(accounts: AccountRow[], ref: string): AccountRow {
  return resolveAccount(accounts, ref, '', t)
}

const setProviderOrder: AssistantTool = {
  id: 'set_provider_order', label: t('调整提供商顺位'), kind: 'write',
  desc: '调整某个提供商在账号全局队列里的**整组顺位**（第几组），决定路由时先试哪一家。这一家的所有账号会作为一整块移动。位置从 1 开始数。',
  params: {
    type: 'object',
    properties: {
      provider: { type: 'string', description: '提供商 id' },
      position: { type: 'number', description: '目标组顺位（1 = 最前）' },
    },
    required: ['provider', 'position'],
  },
  plan: args => ({
    lead: t('将把「{provider}」整组移到第 {position} 顺位：', {
      provider: str(args.provider) || t('（未指定）'), position: String(num(args.position) ?? '?'),
    }),
    note: t('队列是一次性整表提交（后端按新顺序重编优先级），移动会连带调整其它组的相对次序。'),
  }),
  run: async (args, context) => {
    checkToolSignal(context)
    const provider = str(args.provider).toLowerCase()
    const position = num(args.position)
    if (!provider || position === undefined) throw new Error(t('参数不完整（需要 provider / position）'))
    const accounts = await readAccounts()
    const groups = providerGroups(accounts)
    const from = groups.findIndex(group => group.provider === provider)
    if (from < 0) throw new Error(t('没有找到提供商「{provider}」的账号', { provider }))
    const to = Math.max(0, Math.min(groups.length - 1, Math.round(position) - 1))
    const [moved] = groups.splice(from, 1)
    groups.splice(to, 0, moved)
    checkToolSignal(context)
    await bridge().reorderAccounts(groups.flatMap(group => group.ids))
    refreshApp()
    return {
      text: JSON.stringify({ ok: true, provider, from: from + 1, to: to + 1, moved: moved.ids.length }),
      summary: t('已移到第 {n} 顺位', { n: to + 1 }),
    }
  },
}

const setAccountOrder: AssistantTool = {
  id: 'set_account_order', label: t('调整账号优先级'), kind: 'write',
  desc: '把某个账号移动到账号全局队列的第 N 顺位（1 = 最前）。位置决定主备与切换顺序：越靠前越先被使用。',
  params: {
    type: 'object',
    properties: {
      account: { type: 'string', description: '账号名或账号 id' },
      position: { type: 'number', description: '目标顺位（1 = 最前）' },
    },
    required: ['account', 'position'],
  },
  plan: args => ({
    lead: t('将把账号「{account}」移到第 {position} 顺位：', {
      account: str(args.account) || t('（未指定）'), position: String(num(args.position) ?? '?'),
    }),
    note: t('队列是一次性整表提交；位置越靠前，路由时越先被选中。'),
  }),
  run: async (args, context) => {
    checkToolSignal(context)
    const ref = str(args.account, 128)
    const position = num(args.position)
    if (!ref || position === undefined) throw new Error(t('参数不完整（需要 account / position）'))
    const accounts = await readAccounts()
    const target = findAccount(accounts, ref)
    const id = str(target.id)
    const ids = accounts.map(row => str(row.id)).filter(Boolean)
    const from = ids.indexOf(id)
    if (from < 0) throw new Error(t('账号不在当前队列里'))
    ids.splice(from, 1)
    const to = Math.max(0, Math.min(ids.length, Math.round(position) - 1))
    ids.splice(to, 0, id)
    checkToolSignal(context)
    await bridge().reorderAccounts(ids)
    refreshApp()
    return {
      text: JSON.stringify({ ok: true, account: accountLabel(target), from: from + 1, to: to + 1 }),
      summary: t('已移到第 {n} 顺位', { n: to + 1 }),
    }
  },
}

const setAccountEnabled: AssistantTool = {
  id: 'set_account_enabled', label: t('启用 / 禁用账号'), kind: 'write',
  desc: '开启或关闭某几个账号的转发资格。禁用只是把它移出队列（余额仍可查），但**如果某条线路只剩它**，禁用会直接断供。一次可以改多个。',
  params: {
    type: 'object',
    properties: {
      items: {
        type: 'array', description: '要改的账号列表',
        items: {
          type: 'object',
          properties: {
            account: { type: 'string', description: '账号名或账号 id' },
            enabled: { type: 'boolean', description: 'true = 启用，false = 禁用' },
          },
          required: ['account', 'enabled'],
        },
      },
    },
    required: ['items'],
  },
  plan: args => {
    const items = list(args.items)
    return {
      lead: t('将修改 {n} 个账号的启用状态（其中禁用 {k} 个）：', { n: items.length, k: items.filter(item => item.enabled === false).length }),
      table: {
        head: [t('账号'), t('改成')],
        rows: items.map(item => [str(item.account), item.enabled === false ? t('禁用') : t('启用')]),
      },
      note: t('禁用会把账号移出转发队列；若这条线路没有别的可用账号，请求会直接失败。'),
    }
  },
  run: async (args, context) => {
    checkToolSignal(context)
    const api = bridge()
    const accounts = await readAccounts()
    const result = await runToolBatch(list(args.items), context, async item => {
      if (typeof item.enabled !== 'boolean') throw new Error(t('参数不完整（需要 account / enabled）'))
      const target = findAccount(accounts, str(item.account, 128))
      await api.updateAccount(str(target.id), { enabled: item.enabled })
      return `${item.enabled ? '启用' : '禁用'} ${accountLabel(target)}`
    }, t)
    refreshApp()
    return result
  },
}

export const ASSISTANT_ACCOUNT_TOOLS: AssistantTool[] = [setProviderOrder, setAccountOrder, setAccountEnabled]
