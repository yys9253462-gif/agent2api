import type { ImageToolContext } from './assistant-image-tools'
import type { Translate } from './assistant-read-types'
import { safeText } from './assistant-read-data'

type OperationResult = { text: string; summary: string }

export function checkToolSignal(context?: ImageToolContext): void {
  if (context?.signal?.aborted) throw new DOMException('Aborted', 'AbortError')
}

export async function runToolBatch<T>(
  items: readonly T[],
  context: ImageToolContext | undefined,
  execute: (item: T) => Promise<string>,
  t: Translate,
): Promise<OperationResult> {
  if (!items.length) throw new Error(t('没有提供要修改的项目'))
  checkToolSignal(context)
  const changed: string[] = []
  let error = ''
  let cancelled = false
  for (const item of items) {
    if (context?.signal?.aborted) { cancelled = true; break }
    try {
      changed.push(await execute(item))
    } catch (failure) {
      cancelled = failure instanceof DOMException && failure.name === 'AbortError'
      if (!cancelled) error = safeText(failure instanceof Error ? failure.message : String(failure))
      break
    }
  }
  cancelled ||= context?.signal?.aborted === true
  const complete = changed.length === items.length
  return {
    text: JSON.stringify({ ok: complete && !error, changed, remaining: items.length - changed.length, cancelled, ...(error ? { error } : {}) }),
    summary: complete ? t('已完成 {n} 项改动', { n: changed.length })
      : t('已完成 {n} 项，剩余 {m} 项未执行', { n: changed.length, m: items.length - changed.length }),
  }
}

export function providerGroups(accounts: Record<string, unknown>[]): Array<{ provider: string; ids: string[] }> {
  const groups = new Map<string, string[]>()
  for (const account of accounts) {
    if (typeof account.id !== 'string' || !account.id) continue
    const provider = String(account.provider || '').trim().toLowerCase()
    const members = groups.get(provider) || []
    members.push(account.id)
    groups.set(provider, members)
  }
  return [...groups].map(([provider, ids]) => ({ provider, ids }))
}
