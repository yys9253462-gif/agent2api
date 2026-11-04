import { ApiError, authHeaders, refreshPanelAuthIfExpiring, resolveUrl } from './client'
import { serverIdentity } from '../lib/servers'

export async function loadAssistantImage(id: string, signal?: AbortSignal): Promise<Blob> {
  const identity = serverIdentity()
  await refreshPanelAuthIfExpiring()
  if (serverIdentity() !== identity) throw new DOMException('Aborted', 'AbortError')
  const response = await fetch(resolveUrl(`/api/assistant/images/files/${encodeURIComponent(id)}`), { headers: authHeaders(), signal })
  if (!response.ok) throw new ApiError(`HTTP ${response.status}`, response.status)
  const blob = await response.blob()
  if (serverIdentity() !== identity) throw new DOMException('Aborted', 'AbortError')
  return blob
}
