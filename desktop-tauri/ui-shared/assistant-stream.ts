export type ToolCall = { id: string; name: string; arguments: string }
export type ChatEvent =
  | { type: 'delta'; text: string }
  | { type: 'toolCalls'; calls: ToolCall[] }
  | { type: 'done'; finishReason: string }

type Options = {
  onEvent: (event: ChatEvent) => void
  signal?: AbortSignal
  incompleteMessage: string
}

function responseError(chunk: Record<string, unknown>): Error | null {
  const raw = chunk.error
  if (raw === undefined || raw === null) return null
  if (typeof raw === 'string') return new Error(raw)
  const error = raw && typeof raw === 'object' ? raw as Record<string, unknown> : {}
  return new Error(typeof error.message === 'string' ? error.message : JSON.stringify(raw))
}

function callsFrom(raw: unknown): ToolCall[] {
  if (!Array.isArray(raw)) return []
  return raw.map(value => {
    const item = value as Record<string, unknown>
    const fn = item?.function as Record<string, unknown> | undefined
    return {
      id: typeof item?.id === 'string' ? item.id : '',
      name: typeof fn?.name === 'string' ? fn.name : '',
      arguments: typeof fn?.arguments === 'string' ? fn.arguments : '',
    }
  })
}

function finish(calls: ToolCall[], finishReason: string, options: Options): void {
  if (options.signal?.aborted) throw new DOMException('Aborted', 'AbortError')
  if (!finishReason || calls.some(call => !call.id || !call.name)
    || (calls.length && finishReason !== 'tool_calls' && finishReason !== 'stop')) {
    throw new Error(options.incompleteMessage)
  }
  for (const call of calls) {
    let args: unknown
    try { args = JSON.parse(call.arguments || '{}') } catch { throw new Error(options.incompleteMessage) }
    if (!args || typeof args !== 'object' || Array.isArray(args)) throw new Error(options.incompleteMessage)
  }
  if (calls.length) options.onEvent({ type: 'toolCalls', calls })
  options.onEvent({ type: 'done', finishReason })
}

export async function readAssistantStream(response: Response, options: Options): Promise<void> {
  if (options.signal?.aborted) throw new DOMException('Aborted', 'AbortError')
  if (response.headers.get('content-type')?.includes('application/json')) {
    const chunk = await response.json() as Record<string, unknown>
    const error = responseError(chunk)
    if (error) throw error
    const choice = Array.isArray(chunk.choices) ? chunk.choices[0] as Record<string, unknown> | undefined : undefined
    if (!choice) throw new Error(options.incompleteMessage)
    const message = choice.message as Record<string, unknown> | undefined
    if (typeof message?.content === 'string' && message.content) options.onEvent({ type: 'delta', text: message.content })
    finish(callsFrom(message?.tool_calls), String(choice.finish_reason ?? ''), options)
    return
  }
  if (!response.body) throw new Error(options.incompleteMessage)
  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  const calls = new Map<number, ToolCall>()
  let buffer = ''
  let finished = false
  let finishReason = ''

  const handleFrame = (frame: string): void => {
    const data = frame.split(/\r?\n/).filter(line => line.startsWith('data:'))
      .map(line => line.slice(5).trimStart()).join('\n').trim()
    if (!data) return
    if (data === '[DONE]') { finished = true; return }
    let chunk: Record<string, unknown>
    try { chunk = JSON.parse(data) as Record<string, unknown> } catch { throw new Error(options.incompleteMessage) }
    const error = responseError(chunk)
    if (error) throw error
    const choice = Array.isArray(chunk.choices) ? chunk.choices[0] as Record<string, unknown> | undefined : undefined
    const delta = choice?.delta as Record<string, unknown> | undefined
    if (typeof delta?.content === 'string' && delta.content) options.onEvent({ type: 'delta', text: delta.content })
    if (Array.isArray(delta?.tool_calls)) {
      for (const raw of delta.tool_calls as Record<string, unknown>[]) {
        const index = Number(raw.index ?? 0)
        if (!Number.isSafeInteger(index) || index < 0) throw new Error(options.incompleteMessage)
        const slot = calls.get(index) || { id: '', name: '', arguments: '' }
        if (typeof raw.id === 'string' && raw.id) slot.id = raw.id
        const fn = raw.function as Record<string, unknown> | undefined
        if (typeof fn?.name === 'string' && fn.name) slot.name = fn.name
        if (typeof fn?.arguments === 'string') slot.arguments += fn.arguments
        calls.set(index, slot)
      }
    }
    if (typeof choice?.finish_reason === 'string') finishReason = choice.finish_reason
  }

  try {
    while (!finished) {
      const { done, value } = await reader.read()
      if (options.signal?.aborted) throw new DOMException('Aborted', 'AbortError')
      buffer += done ? decoder.decode() : decoder.decode(value, { stream: true })
      let boundary = /\r?\n\r?\n/.exec(buffer)
      while (boundary && !finished) {
        const frame = buffer.slice(0, boundary.index)
        buffer = buffer.slice(boundary.index + boundary[0].length)
        handleFrame(frame)
        boundary = /\r?\n\r?\n/.exec(buffer)
      }
      if (done) {
        if (!finished && buffer.trim()) handleFrame(buffer)
        break
      }
    }
    if (!finished && !finishReason) throw new Error(options.incompleteMessage)
    const completed = [...calls.entries()].sort((a, b) => a[0] - b[0]).map(([, call]) => call)
    finish(completed, finishReason, options)
  } finally {
    await reader.cancel().catch(() => {})
    reader.releaseLock()
  }
}
