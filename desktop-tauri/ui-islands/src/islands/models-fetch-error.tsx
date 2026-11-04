import * as React from 'react'
import { Alert, AlertTitle, Button } from '@ui'
import { t } from '../i18n'

function responseOf(message: string): { status?: string; body: string; upstream: boolean } {
  // 管理接口保留「上游返回 {status}: {正文}」格式，桥接层只传递错误字符串。
  const upstream = /^上游返回\s+([1-5]\d{2})[:：] ?/.exec(message)
  if (upstream) return { status: upstream[1], body: message.slice(upstream[0].length), upstream: true }
  const http = /^HTTP\s+([1-5]\d{2})(?:\b|$)/i.exec(message)
  return { status: http?.[1], body: message, upstream: false }
}

export function ModelsFetchError({ message }: { message: string }) {
  const { status, body, upstream } = responseOf(message)
  const bytes = new TextEncoder().encode(body).length
  const title = status ? t('{status}：模型获取失败', { status }) : t('模型获取失败')
  const label = upstream ? t('上游原始响应') : t('错误详情')

  return (
    <div className='flex min-w-0 flex-col gap-3'>
      <Alert variant='destructive' className='flex items-center gap-2 rounded-none border-0 bg-transparent p-0'>
        <span aria-hidden='true' className='size-[7px] shrink-0 rounded-full bg-current' />
        <AlertTitle className='text-[13px]'>{title}</AlertTitle>
      </Alert>
      <div className='min-w-0 overflow-hidden rounded-sm border border-border bg-surface-2'>
        <div className='flex items-center gap-2 border-b border-hairline py-[7px] pl-3 pr-2.5'>
          <span className='text-[11.5px] font-semibold text-subtle'>{label}</span>
          <span className='font-mono text-[10.5px] text-muted-foreground'>{t('{n} 字节', { n: bytes })}</span>
          <Button variant='outline' size='xs' className='ml-auto h-[22px] text-[11px]'
            data-copy={body} disabled={!body}>
            {t('复制')}
          </Button>
        </div>
        <pre role='region' aria-label={label} tabIndex={0}
          className='m-0 max-h-[210px] overflow-auto whitespace-pre-wrap px-3 py-[11px] font-mono text-[11.5px] leading-[1.7] text-subtle [overflow-wrap:anywhere] focus-visible:outline-none focus-visible:shadow-focus'>
          {body || (upstream ? t('（上游这一次返回了空正文）') : '—')}
        </pre>
      </div>
    </div>
  )
}
