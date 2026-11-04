import * as React from 'react'
import { t } from '../i18n'
import { loadAssistantConfig, saveAssistantConfig, toast, type AssistantConfig } from './assistant-state'

export function useAssistantConfig() {
  const [config, setConfig] = React.useState<AssistantConfig | null>(null)
  const [status, setStatus] = React.useState('')
  const [failed, setFailed] = React.useState(false)
  const configRef = React.useRef<AssistantConfig | null>(null)
  const mountedRef = React.useRef(true)
  const loadedRef = React.useRef(false)
  const loadingRef = React.useRef<Promise<void> | null>(null)
  const savingRef = React.useRef<Promise<void>>(Promise.resolve())
  const revisionRef = React.useRef(0)
  const retryRef = React.useRef(0)
  const timerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)

  const update = React.useCallback((next: AssistantConfig) => {
    configRef.current = next
    setConfig(next)
  }, [])

  const load = React.useCallback((): Promise<void> => {
    if (loadedRef.current) return Promise.resolve()
    if (loadingRef.current) return loadingRef.current
    const revision = revisionRef.current
    loadingRef.current = loadAssistantConfig().then(next => {
      loadedRef.current = true
      retryRef.current = 0
      if (!mountedRef.current || revision !== revisionRef.current) return
      update(next)
      setFailed(false)
      setStatus('')
    }).catch(error => {
      if (!mountedRef.current || revision !== revisionRef.current) return
      setFailed(true)
      setStatus(t('加载助手设置失败：{message}', { message: error instanceof Error ? error.message : String(error) }))
      if (retryRef.current < 3) {
        retryRef.current += 1
        timerRef.current = setTimeout(() => {
          timerRef.current = null
          if (mountedRef.current) void load()
        }, 4000)
      }
    }).finally(() => { loadingRef.current = null })
    return loadingRef.current
  }, [update])

  const save = React.useCallback((next: AssistantConfig, note?: string) => {
    const revision = ++revisionRef.current
    update(next)
    // 整块配置必须串行保存，较早的响应不能覆盖后来的模型选择。
    savingRef.current = savingRef.current.then(async () => {
      try {
        const saved = await saveAssistantConfig(next)
        if (!mountedRef.current || revision !== revisionRef.current) return
        update(saved)
        setFailed(false)
        setStatus('')
        if (note) toast(note)
      } catch (error) {
        if (!mountedRef.current || revision !== revisionRef.current) return
        setFailed(true)
        setStatus(t('保存失败：{message}', { message: error instanceof Error ? error.message : String(error) }))
      }
    })
  }, [update])

  React.useEffect(() => {
    mountedRef.current = true
    return () => {
      mountedRef.current = false
      if (timerRef.current) clearTimeout(timerRef.current)
    }
  }, [])

  return { config, configRef, status, failed, load, save }
}
