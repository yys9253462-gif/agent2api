import { useCallback, useEffect, useRef, useState } from 'react'
import { apiCall } from '../api/client'
import { serverIdentity } from '../lib/servers'
import { usePolling } from '../lib/usePolling'
import type { PluginProviderDocumentation, ProviderDocsPayload } from '../../../ui-shared/provider-docs'

export function useProviderDocumentation() {
  const identity = serverIdentity()
  const [snapshot, setSnapshot] = useState<{ identity: string; packs: PluginProviderDocumentation[]; error: string }>({
    identity, packs: [], error: '',
  })
  const controller = useRef<AbortController | null>(null)

  const load = useCallback(async () => {
    controller.current?.abort()
    const request = new AbortController()
    controller.current = request
    try {
      const data = await apiCall<ProviderDocsPayload>('/api/plugins', { signal: request.signal })
      if (request.signal.aborted || serverIdentity() !== identity) return
      setSnapshot({ identity, packs: Array.isArray(data.providerDocs) ? data.providerDocs : [], error: '' })
    } catch (cause) {
      if (request.signal.aborted || serverIdentity() !== identity) return
      setSnapshot(previous => ({
        identity,
        packs: previous.identity === identity ? previous.packs : [],
        error: cause instanceof Error ? cause.message : String(cause),
      }))
    }
  }, [identity])

  const { refreshing, refresh } = usePolling(load, 20_000, [identity])
  useEffect(() => () => controller.current?.abort(), [identity])
  return {
    packs: snapshot.identity === identity ? snapshot.packs : [],
    error: snapshot.identity === identity ? snapshot.error : '',
    loading: refreshing,
    refresh,
  }
}
