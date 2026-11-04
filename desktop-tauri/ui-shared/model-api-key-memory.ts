import { useCallback, useEffect, useState } from 'react'

type Purpose = 'model-test' | 'assistant'
const memory = new Map<string, string | null>()

function storageKey(scope: string, purpose: Purpose): string {
  return `agent2api.model-api-key.${purpose}.${encodeURIComponent(scope)}`
}

function keyId(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const id = value.trim()
  return id && id.length <= 128 ? id : null
}

export function readModelApiKeySelection(scope: string, purpose: Purpose): string | null {
  if (!scope) return null
  const key = storageKey(scope, purpose)
  if (memory.has(key)) return memory.get(key) ?? null
  let id: string | null = null
  try { id = keyId(localStorage.getItem(key)) } catch {}
  memory.set(key, id)
  return id
}

export function rememberModelApiKeySelection(scope: string, purpose: Purpose, value: string | null | undefined): void {
  if (!scope) return
  const key = storageKey(scope, purpose)
  const id = keyId(value)
  memory.set(key, id)
  try {
    if (id) localStorage.setItem(key, id)
    else localStorage.removeItem(key)
  } catch {}
}

export function useRememberedModelApiKey(purpose: Purpose, getScope: () => Promise<string>) {
  const [scope, setScope] = useState<string | null>(null)
  const [value, setValue] = useState<string | null>(null)
  useEffect(() => {
    let active = true
    setScope(null); setValue(null)
    void getScope().then(scope => {
      if (!active) return
      setScope(scope)
      setValue(readModelApiKeySelection(scope, purpose))
    }).catch(() => { if (active) setScope('') })
    return () => { active = false }
  }, [getScope, purpose])
  const select = useCallback((id: string) => {
    setValue(id)
    if (scope) rememberModelApiKeySelection(scope, purpose, id)
  }, [scope, purpose])
  return { value, ready: scope !== null, select }
}
