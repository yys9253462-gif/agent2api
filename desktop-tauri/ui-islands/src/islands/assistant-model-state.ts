import * as React from 'react'
import { loadModelOptions } from './assistant-tools'
import { modelOptionsFrom, type ModelOption } from './assistant-models'
import { callJson } from './assistant-state'

export function useAssistantModels(apiKey?: string) {
  const [models, setModels] = React.useState<ModelOption[]>([])
  const [ready, setReady] = React.useState(false)
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState('')
  const taskRef = React.useRef<Promise<ModelOption[]> | null>(null)
  const revisionRef = React.useRef(0)
  const mountedRef = React.useRef(true)

  const refresh = React.useCallback((): Promise<ModelOption[]> => {
    if (taskRef.current) return taskRef.current
    setLoading(true)
    const revision = revisionRef.current
    const task = apiKey ? callJson<{ data: unknown[] }>('GET', '/api/model-access/models', undefined, undefined, apiKey)
      .then(data => modelOptionsFrom(data.data)) : loadModelOptions()
    taskRef.current = task.then(options => {
      if (mountedRef.current && revision === revisionRef.current) {
        setModels(options)
        setReady(true)
        setError('')
      }
      return options
    }).catch(failure => {
      if (mountedRef.current && revision === revisionRef.current) {
        setReady(false)
        setError(failure instanceof Error ? failure.message : String(failure))
      }
      throw failure
    }).finally(() => {
      if (revision !== revisionRef.current) return
      taskRef.current = null
      if (mountedRef.current) setLoading(false)
    })
    return taskRef.current
  }, [apiKey])

  React.useEffect(() => {
    revisionRef.current += 1
    taskRef.current = null
    setReady(false)
    void refresh().catch(() => {})
  }, [refresh])

  React.useEffect(() => {
    mountedRef.current = true
    const refreshVisible = () => {
      if (document.hidden || !document.querySelector('.page.active[data-page="assistant"]')) return
      void refresh().catch(() => {})
    }
    const interval = window.setInterval(refreshVisible, 20_000)
    document.addEventListener('visibilitychange', refreshVisible)
    return () => {
      mountedRef.current = false
      window.clearInterval(interval)
      document.removeEventListener('visibilitychange', refreshVisible)
    }
  }, [refresh])

  return { models, ready, loading, error, refresh }
}
