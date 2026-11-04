import type { CSSProperties } from 'react'
import { Icon } from '../../components/Icon'

export function AssistantAvatar() {
  return <span style={{
    width: 26, height: 26, borderRadius: 8, flex: 'none', display: 'grid', placeItems: 'center', background: 'var(--m-primary-soft)', color: 'var(--m-primary)',
  }}>
    <Icon name='spark' style={{ width: 15, height: 15 }} />
  </span>
}

export function assistantBubbleStyle(mine: boolean): CSSProperties {
  return {
    maxWidth: '82%', padding: '10px 13px', borderRadius: mine ? '14px 14px 5px 14px' : '14px 14px 14px 5px',
    background: mine ? 'var(--m-primary)' : 'var(--m-surface)', color: mine ? '#fff' : 'var(--m-text)',
    border: mine ? 0 : '1px solid var(--m-line)', fontSize: 13.5, lineHeight: 1.62,
  }
}

export function parseAssistantArgs(raw: string): Record<string, unknown> {
  const text = String(raw || '').trim()
  if (!text) return {}
  try {
    const parsed = JSON.parse(text)
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed as Record<string, unknown> : {}
  } catch { return {} }
}
