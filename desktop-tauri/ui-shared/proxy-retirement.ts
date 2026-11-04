export const PROXY_RETIREMENT_KEY = 'agent2api:proxy-subscription-retired:v1'

export function shouldShowProxyRetirement(scope = ''): boolean {
  try {
    return localStorage.getItem(`${PROXY_RETIREMENT_KEY}:${scope}`) !== 'acknowledged'
  } catch {
    return true
  }
}

export function acknowledgeProxyRetirement(scope = ''): void {
  try {
    localStorage.setItem(`${PROXY_RETIREMENT_KEY}:${scope}`, 'acknowledged')
  } catch {}
}
