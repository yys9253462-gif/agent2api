import * as React from 'react'
import { Badge, Button, Spinner } from '@ui'
import { t } from '../i18n'

export type TestModelResponse = {
  success?: boolean
  status?: number
  error?: string | null
  reply?: string
  reasoning?: string
  provider?: string
  model?: string
  protocol?: string
  proxy?: string | null
  account_id?: string | null
  account_name?: string | null
  upstream_model?: string | null
  upstream_reasoning?: string | null
  duration_ms?: number
  ttfb_ms?: number | null
  attempts?: number
  prompt_tokens?: number
  completion_tokens?: number
  total_tokens?: number
  apiKeyId?: string
  apiKeyName?: string
  apiKeySource?: string
}

export type ModelTestSlot = {
  accountId: string
  label: string
  position: number
  testId: string
  startedAt: number
  state: 'running' | 'done'
  result?: TestModelResponse
  transport?: string
}

export function modelTestOk(slot: ModelTestSlot): boolean {
  if (slot.transport || !slot.result || slot.result.error) return false
  const status = Number(slot.result.status) || 0
  return status >= 200 && status < 300
}

function formatMs(value: unknown): string {
  const time = Number(value)
  if (!Number.isFinite(time) || time <= 0) return '—'
  return time < 1000 ? `${Math.round(time)} ms` : `${(time / 1000).toFixed(time < 10000 ? 2 : 1)} s`
}

const PROTOCOL_SHORT: Record<string, string> = {
  chat_completions: 'Chat Completions', responses: 'Responses', anthropic: 'Anthropic Messages',
}

const STATUS_HINT: Array<{ test: (status: number) => boolean; hint: string }> = [
  { test: status => status === 429, hint: t('该账号对这个模型正在限额冷却，等一会儿自己会好；也可以先测别的账号。') },
  { test: status => status === 401 || status === 403, hint: t('上游拒绝了这条登录态：重新登录一次，或把这条账号删掉。') },
  { test: status => status === 404, hint: t('上游不认识这个模型名：先点「获取模型」刷新清单，或核对默认绑定指向的上游模型。') },
  { test: status => status === 504, hint: t('上游没在预算内给完回答。可以只勾一个账号再测一次，看是普遍慢还是某一个账号慢。') },
  { test: status => status >= 500, hint: t('多半是上游自己的问题，过一会儿再试。同一个错误出现在所有账号上时，先查模型目录与映射。') },
]

export function ModelTestResult({ slot, model, now, expanded, onToggle }: {
  slot: ModelTestSlot; model: string; now: number; expanded: boolean; onToggle: () => void
}) {
  const head = (badge: React.ReactNode, metrics: string) => (
    <div className='flex items-center gap-2.5 border-b border-hairline px-3 py-2'>
      {badge}
      <b className='min-w-0 truncate text-[12.5px] text-foreground' title={slot.label}>{slot.label}</b>
      {slot.position ? <span className='flex-none text-xs text-subtle'>#{slot.position}</span> : null}
      <span className='ml-auto flex-none font-mono text-[11.5px] tabular-nums text-subtle'>{metrics}</span>
    </div>
  )
  if (slot.state === 'running') return (
    <div className='flex items-center gap-2.5 rounded-md border border-border bg-surface-2 px-3 py-2.5 text-xs text-subtle'>
      <Spinner className='flex-none' />
      <span className='min-w-0 truncate'>{t('账号 ')}<b className='text-foreground'>{slot.label}</b>{t(' 已发出，等待上游首帧…')}</span>
      <span className='ml-auto flex-none font-mono tabular-nums'>{t('已等待 {seconds}s', { seconds: ((now - slot.startedAt) / 1000).toFixed(1) })}</span>
    </div>
  )
  const result = slot.result
  if (!modelTestOk(slot)) {
    const status = Number(result?.status) || 0
    const hint = STATUS_HINT.find(item => item.test(status))?.hint ?? (slot.transport ? t('确认桌面端还在运行，再重试一次。') : '')
    return (
      <div className='rounded-md border border-border bg-surface-2'>
        {head(<Badge shape='tag' variant='destructive' className='flex-none'>{status || t('失败')}</Badge>, t('用时 {time}', { time: formatMs(result?.duration_ms) }))}
        <div className='flex flex-col gap-1.5 px-3 py-2.5'>
          <div className='text-[12.5px] leading-[1.7]'>
            {slot.transport
              ? <><b className='text-destructive'>{t('本机网关没有返回结论')}</b>{t('：')}{slot.transport}</>
              : <><b className='text-destructive'>{t('测试未通过')}</b>{t('：')}{result?.error || t('上游没有给出可读的错误说明')}</>}
            {Number(result?.attempts) > 1 ? t('（中间共尝试 {n} 次）', { n: Number(result?.attempts) }) : ''}
          </div>
          {hint ? <p className='text-xs leading-[1.65] text-subtle'>{hint}</p> : null}
        </div>
      </div>
    )
  }
  const reply = String(result?.reply ?? '')
  const reasoning = String(result?.reasoning ?? '')
  const meta = [
    result?.upstream_model && result.upstream_model !== model ? t('上游 {model}', { model: result.upstream_model }) : '',
    result?.upstream_reasoning ? t('思考等级 {level}', { level: result.upstream_reasoning }) : '',
    result?.protocol && result.protocol !== 'chat_completions' ? PROTOCOL_SHORT[result.protocol] || result.protocol : '',
    result?.proxy ? t('节点 {proxy}', { proxy: result.proxy }) : '',
    result?.apiKeyName ? `API Key: ${result.apiKeyName}` : '',
    Number(result?.attempts) > 1 ? t('尝试 {n} 次', { n: Number(result?.attempts) }) : '',
  ].filter(Boolean).join(' · ')
  return (
    <div className='rounded-md border border-border bg-surface-2'>
      {head(<Badge shape='tag' variant='success' className='flex-none'>{Number(result?.status) || 200}</Badge>,
        t('用时 {time} · 首字 {ttfb}', { time: formatMs(result?.duration_ms), ttfb: formatMs(result?.ttfb_ms) }))}
      <div className='flex flex-col gap-2 px-3 py-2.5'>
        <div className='flex items-center gap-2 text-[11.5px] text-subtle'>
          <span className='flex-none'>{t('回复')}</span>
          {meta ? <span className='min-w-0 truncate' title={meta}>{meta}</span> : null}
          <Button variant='ghost' size='2xs' className='ml-auto flex-none' data-copy={reply}>{t('复制')}</Button>
        </div>
        <div className='max-h-[220px] overflow-y-auto whitespace-pre-wrap rounded-sm border border-border bg-surface px-3 py-2 text-[12.5px] leading-[1.7]'>
          {reply || <span className='text-subtle'>{t('（上游这一次返回了空正文）')}</span>}
        </div>
        {reasoning ? <div className='overflow-hidden rounded-sm border border-border'>
          <Button variant='ghost' size='xs' className='w-full' aria-expanded={expanded} onClick={onToggle}>
            <span className='truncate'>{t('思考过程（{n} 字）', { n: reasoning.length })}</span>
            <span aria-hidden='true' className='ml-auto text-[10px]'>{expanded ? '▴' : '▾'}</span>
          </Button>
          {expanded ? <div className='border-t border-hairline bg-surface px-2.5 py-2'>
            <pre className='whitespace-pre-wrap font-mono text-[11.5px] leading-[1.65] text-subtle'>{reasoning}</pre>
          </div> : null}
        </div> : null}
      </div>
    </div>
  )
}
