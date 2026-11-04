import type { RoutingWorkspace } from './routing-types'

export function inactiveProvider(workspace: RoutingWorkspace, provider: string): boolean {
  if (workspace.profile.disabledProviders.includes(provider)) return true
  if (workspace.providers.find(item => item.id === provider)?.available === false) return true
  const ids = workspace.profile.accountOrder[provider] || []
  const participants = workspace.accounts.filter(account => ids.includes(account.id) && !workspace.profile.excludedAccounts.includes(account.id))
  return !participants.length || participants.every(account => account.enabled === false)
}

export function orderedProviders(workspace: RoutingWorkspace): { active: string[]; inactive: string[] } {
  const active: string[] = [], inactive: string[] = []
  for (const provider of workspace.profile.providers) {
    (inactiveProvider(workspace, provider) ? inactive : active).push(provider)
  }
  return { active, inactive }
}
