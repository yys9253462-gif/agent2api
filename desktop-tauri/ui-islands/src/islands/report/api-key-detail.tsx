import * as React from 'react'
import { Badge, Button, Dialog, DialogBody, DialogContent, DialogHeader, DialogTitle, Empty, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@ui'
import { formatInt, formatTokens, type CustomRange, type StatsGroup, type StatsSummary } from '../report-charts'
import { openRequestLogs, sourceName } from './source-stats'
import { t } from '../../i18n'

export function ApiKeyUsageDialog({ target, range, dates, onClose }: {
  target: StatsGroup; range: string; dates?: CustomRange; onClose: () => void
}) {
  const [data, setData] = React.useState<StatsSummary | null>(null)
  const [error, setError] = React.useState('')
  const filterValue = target.filterValue || 'unknown'
  React.useEffect(() => {
    let cancelled = false
    setData(null)
    setError('')
    const api = (window as unknown as { workbuddyDesktop?: {
      getStatsSummary(range: string, dates?: CustomRange, options?: { apiKey?: string }): Promise<StatsSummary>
    } }).workbuddyDesktop
    if (!api) { setError(t('后端桥不可用')); return }
    void api.getStatsSummary(range, dates, { apiKey: filterValue }).then(value => {
      if (!cancelled) setData(value)
    }).catch(failure => {
      if (!cancelled) setError(failure instanceof Error ? failure.message : String(failure))
    })
    return () => { cancelled = true }
  }, [range, dates?.start, dates?.end, filterValue])
  const detail = data?.apiKeyDetail
  const showRows = (rows: StatsGroup[] | undefined, clients: boolean) => {
    const total = Number(detail?.totalTokens) || 0
    return rows?.length ? (
      <Table><TableHeader><TableRow>
        <TableHead>{clients ? t('客户端') : t('模型')}</TableHead>
        <TableHead className='text-right'>{t('请求数')}</TableHead>
        <TableHead className='text-right'>{t('总 Token')}</TableHead>
        <TableHead className='text-right'>{t('占比')}</TableHead>
        <TableHead className='text-right'>{t('操作')}</TableHead>
      </TableRow></TableHeader><TableBody>{rows.map((row, index) => (
        <TableRow key={row.id || row.model || index}>
          <TableCell>{clients ? sourceName(row, 'clients') : row.label || row.model || t('未知模型')}</TableCell>
          <TableCell className='text-right'>{formatInt(row.requests)}</TableCell>
          <TableCell className='text-right'>{formatTokens(row.totalTokens)}</TableCell>
          <TableCell className='text-right'>{total ? `${((Number(row.totalTokens) || 0) / total * 100).toFixed(1)}%` : '—'}</TableCell>
          <TableCell className='text-right'><Button variant='ghost' size='sm' onClick={() => {
            openRequestLogs({ apiKey: filterValue, startDate: data?.startDate, endDate: data?.endDate,
              ...(clients ? { client: row.id || '' } : { model: row.model || '' }) })
            onClose()
          }}>{t('请求日志')}</Button></TableCell>
        </TableRow>
      ))}</TableBody></Table>
    ) : <Empty>{t('没有可恢复的维度明细')}</Empty>
  }
  return (
    <Dialog open onOpenChange={open => { if (!open) onClose() }}>
      <DialogContent className='max-w-[900px]'>
        <DialogHeader><DialogTitle>{t('API Key 用量详情')} · {sourceName(target, 'apiKeys')}</DialogTitle></DialogHeader>
        <DialogBody>
          <div className='flex items-center gap-2 text-xs text-muted-foreground'>
            {data?.startDate} ～ {data?.endDate}
            {target.source !== 'key' ? <Badge variant='outline'>{sourceName(target, 'apiKeys')}</Badge> : null}
          </div>
          {error ? <Empty className='text-destructive'>{error}</Empty> : !data ? <Empty>{t('正在加载…')}</Empty> : (
            <>
              {detail?.detailsComplete === false ? <p className='text-xs text-warning'>{t('历史明细不完整，输入、输出与缓存读数无法恢复')}</p> : null}
              <h3 className='mt-4 mb-2 font-semibold'>{t('客户端分布')}</h3>
              {showRows(detail?.clients, true)}
              <h3 className='mt-4 mb-2 font-semibold'>{t('模型分布')}</h3>
              {showRows(detail?.models, false)}
              <p className='mt-3 text-xs text-muted-foreground'>{t('分布来自保留的日报来源明细；无法恢复的历史不反推客户端或模型。')}</p>
            </>
          )}
        </DialogBody>
      </DialogContent>
    </Dialog>
  )
}
