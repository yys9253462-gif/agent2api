import { apiCall } from './client'
import { t } from '../i18n'
import type { UpdateChannels, UpdateCheckPlan } from '../../../ui-shared/updates'

export type { UpdateChannels, UpdateCheckPlan } from '../../../ui-shared/updates'

export function getUpdateChannels(signal?: AbortSignal): Promise<UpdateChannels> {
  return apiCall('/api/update/channels', { signal })
}

export function saveUpdateChannels(body: Partial<Pick<UpdateChannels, 'stable' | 'beta'>>): Promise<UpdateChannels> {
  return apiCall('/api/update/channels', { method: 'POST', body })
}

export async function getUpdateCheckPlan(signal?: AbortSignal): Promise<UpdateCheckPlan> {
  const payload = await apiCall<{ tasks?: Array<UpdateCheckPlan & { id: string }> }>('/api/scheduled-tasks', { signal })
  const task = payload.tasks?.find(task => task.id === 'updateCheck')
  if (!task) throw new Error(t('软件版本检查设置不可用'))
  return task
}

export function saveUpdateCheckPlan(body: Partial<Pick<UpdateCheckPlan, 'enabled' | 'interval'>>): Promise<UpdateCheckPlan> {
  return apiCall('/api/scheduled-tasks/updateCheck', { method: 'PATCH', body })
}
