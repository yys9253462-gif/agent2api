import { useCallback, useEffect, useRef, useState } from 'react'
import { fetchSession } from '../api/gateway'
import { apiCall } from '../api/client'
import { modelOptionsFrom, type ModelOption } from '../../../ui-shared/assistant-models'
import { serverIdentity } from './servers'
import { t } from '../i18n'

export function unavailableModelMessage(name: string): string {
  return t('模型「{model}」当前不在可用列表中，请重新选择；也可到「模型管理」或「插件」页检查是否已启用。', { model: name })
}

export function useAssistantModels(apiKey?: string) {
  const [models, setModels] = useState<ModelOption[]>([])
  const [ready, setReady] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const mounted = useRef(true)
  const taskRef = useRef<{ promise: Promise<ModelOption[]>; controller: AbortController } | null>(null)
  const identity = serverIdentity()

  const refresh = useCallback((): Promise<ModelOption[]> => {
    if (!mounted.current || serverIdentity() !== identity) return Promise.reject(new DOMException('请求已取消', 'AbortError'))
    if (taskRef.current && !taskRef.current.controller.signal.aborted) return taskRef.current.promise
    const controller = new AbortController()
    setLoading(true)
    const source = apiKey
      ? apiCall<{ data: unknown[] }>('/api/model-access/models', { signal: controller.signal, apiKey }).then(data => ({ models: data.data }))
      : fetchSession(controller.signal)
    const promise = source.then(session => {
      if (controller.signal.aborted || !mounted.current || serverIdentity() !== identity) throw new DOMException('请求已取消', 'AbortError')
      if (!Array.isArray(session.models)) throw new Error(t('读取模型清单失败'))
      const options = modelOptionsFrom(session.models)
      if (mounted.current && serverIdentity() === identity) {
        setModels(options)
        setReady(true)
        setError('')
      }
      return options
    }).catch(failure => {
      if (mounted.current && !controller.signal.aborted && serverIdentity() === identity) {
        setReady(false)
        setError(failure instanceof Error ? failure.message : String(failure))
      }
      throw failure
    }).finally(() => {
      if (taskRef.current?.controller === controller) {
        taskRef.current = null
        if (mounted.current && serverIdentity() === identity) setLoading(false)
      }
    })
    taskRef.current = { promise, controller }
    return promise
  }, [identity, apiKey])

  useEffect(() => {
    mounted.current = true
    void refresh().catch(() => {})
    const refreshVisible = () => {
      if (!document.hidden) void refresh().catch(() => {})
    }
    document.addEventListener('visibilitychange', refreshVisible)
    return () => {
      mounted.current = false
      taskRef.current?.controller.abort()
      document.removeEventListener('visibilitychange', refreshVisible)
    }
  }, [refresh])

  return { models, ready, loading, error, refresh }
}
