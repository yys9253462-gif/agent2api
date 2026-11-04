import * as React from 'react'
import { api } from './plugins-shared'
import type { PluginProviderDocumentation, ProviderDocsPayload } from '../../../ui-shared/provider-docs'

const PAGE_SELECTOR = '.page[data-page="docs"]'

type AppRefresh = {
  getRefreshStatus?: () => { pending: boolean; error: string }
  subscribeRefresh?: (listener: () => void) => () => void
}

export function useProviderDocumentation() {
  const [packs, setPacks] = React.useState<PluginProviderDocumentation[]>([])
  const [error, setError] = React.useState('')
  const [loading, setLoading] = React.useState(false)
  const alive = React.useRef(false)
  const sequence = React.useRef(0)

  const load = React.useCallback(async () => {
    const request = ++sequence.current
    if (alive.current) setLoading(true)
    try {
      const data = await api<ProviderDocsPayload>('GET', '/api/plugins')
      if (!alive.current || request !== sequence.current) return
      setPacks(Array.isArray(data.providerDocs) ? data.providerDocs : [])
      setError('')
    } catch (cause) {
      if (alive.current && request === sequence.current) {
        setError(cause instanceof Error ? cause.message : String(cause))
      }
    } finally {
      if (alive.current && request === sequence.current) setLoading(false)
    }
  }, [])

  React.useEffect(() => {
    alive.current = true
    const section = document.querySelector(PAGE_SELECTOR)
    const visible = () => !document.hidden && section?.classList.contains('active') === true
    const refreshVisible = () => { if (visible()) void load() }
    const app = (window as unknown as { wbApp?: AppRefresh }).wbApp
    const unsubscribe = app?.subscribeRefresh?.(() => {
      const status = app.getRefreshStatus?.()
      if (status && !status.pending && !status.error) refreshVisible()
    })
    const observer = new MutationObserver(refreshVisible)
    if (section) observer.observe(section, { attributes: true, attributeFilter: ['class'] })
    document.addEventListener('visibilitychange', refreshVisible)
    // 无宿主订阅时仍可随其它窗口的插件安装与卸载变化。
    const timer = unsubscribe ? null : window.setInterval(refreshVisible, 20_000)
    refreshVisible()
    return () => {
      alive.current = false
      sequence.current += 1
      observer.disconnect()
      unsubscribe?.()
      document.removeEventListener('visibilitychange', refreshVisible)
      if (timer !== null) window.clearInterval(timer)
    }
  }, [load])

  return { packs, error, loading, load }
}
