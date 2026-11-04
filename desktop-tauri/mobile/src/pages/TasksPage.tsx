import { useState } from 'react'
import { PageIntro, IconButton, Empty, ErrorNote, SkeletonList } from '../components/ui'
import { GlyphLogo } from '../components/ProviderLogo'
import { Icon } from '../components/Icon'
import { PullIndicator, usePullToRefresh } from '../components/PullToRefresh'
import { store } from '../state/store'
import {
  fetchScheduledTasks,
  patchScheduledTask,
  runScheduledTask,
  type ScheduledTask,
} from '../api/gateway'
import { formatRelative, formatStamp } from '../lib/format'
import { usePolling } from '../lib/usePolling'
import { t } from '../i18n'
import { Button, Card, InputGroup, InputGroupAddon, InputGroupInput, InputGroupText, Label, Sheet, SheetBody, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle, Switch as UiSwitch } from '@ui'

/**
 * 定时任务。
 *
 * 手机上以「只读 + 开关」为主：改间隔要填数字，窄屏上既费事又容易点错，
 * 收进抽屉且给了上下限提示。后端任务额外给「立即执行」。
 *
 * 与桌面端一致：签到是每天定点型，不在这张表里（它有自己的页）。
 */
export function TasksPage() {
  const [tasks, setTasks] = useState<ScheduledTask[] | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState('')
  const [editing, setEditing] = useState<ScheduledTask | null>(null)
  const [draft, setDraft] = useState('')

  const load = async () => {
    try {
      const data = await fetchScheduledTasks()
      setTasks(data.tasks || [])
      setError('')
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure))
    }
  }

  usePolling(load, 30_000)

  const pull = usePullToRefresh({ onRefresh: load })

  const list = tasks || []
  const backend = list.filter(item => item.runner === 'backend')
  const frontend = list.filter(item => item.runner === 'frontend')
  const enabledCount = list.filter(item => item.enabled).length

  const toggle = async (task: ScheduledTask, next: boolean) => {
    if (!task.id) return
    setBusy(task.id)
    try {
      const updated = await patchScheduledTask(task.id, { enabled: next })
      setTasks(previous => previous?.map(item => item.id === updated.id ? updated : item) || [updated])
      store.toast(t('已{action}「{name}」', {
        action: next ? t('开启') : t('关闭'),
        name: task.label || task.id,
      }))
    } catch (failure) {
      store.toast(failure instanceof Error ? failure.message : String(failure), 'err')
    } finally {
      setBusy('')
    }
  }

  const run = async (task: ScheduledTask) => {
    if (!task.id) return
    setBusy(task.id)
    try {
      await runScheduledTask(task.id)
      store.toast(t('「{name}」已执行', { name: task.label || task.id }))
      await load()
    } catch (failure) {
      store.toast(failure instanceof Error ? failure.message : String(failure), 'err')
    } finally {
      setBusy('')
    }
  }

  const saveInterval = async () => {
    if (!editing?.id) return
    const value = Number(draft)
    if (!Number.isInteger(value) || value < (editing.min || 1) || value > (editing.max || 9999)) {
      store.toast(
        t('间隔必须是 {min}–{max} 的整数', { min: editing.min || 1, max: editing.max || 9999 }),
        'err',
      )
      return
    }
    setBusy(editing.id)
    try {
      const updated = await patchScheduledTask(editing.id, { interval: value })
      setTasks(previous => previous?.map(item => item.id === updated.id ? updated : item) || [updated])
      store.toast(t('已更新间隔'))
      setEditing(null)
    } catch (failure) {
      store.toast(failure instanceof Error ? failure.message : String(failure), 'err')
    } finally {
      setBusy('')
    }
  }

  return (
    <div className='m-page' ref={pull.bindRef}>
      <PullIndicator pull={pull.pull} refreshing={pull.refreshing} />
      <PageIntro actions={<IconButton name='refresh' label={t('刷新')} onClick={() => void load()} />}
      />

      {error ? <ErrorNote text={error} /> : null}
      {!tasks ? <SkeletonList rows={4} height={64} /> : null}

      {tasks ? (
        <>
          <div className='m-section' style={{ marginTop: 0 }}>
            <div className='m-section-head'>
              <h2>{t('后台任务')}</h2>
            </div>
            <Card className='m-card'>
              {backend.length ? (
                backend.map(task => (
                  <TaskRow
                    key={task.id}
                    task={task}
                    busy={busy === task.id}
                    onToggle={next => void toggle(task, next)}
                    onRun={() => void run(task)}
                    onEdit={() => {
                      setEditing(task)
                      setDraft(String(task.interval ?? ''))
                    }}
                  />
                ))
              ) : (
                <Empty text={t('没有后台任务')} />
              )}
            </Card>
          </div>

          {frontend.length ? (
            <div className='m-section'>
              <div className='m-section-head'>
                <h2>{t('页面自动刷新')}</h2>
              </div>
              <Card className='m-card'>
                {frontend.map(task => (
                  <TaskRow
                    key={task.id}
                    task={task}
                    busy={busy === task.id}
                    onToggle={next => void toggle(task, next)}
                  />
                ))}
              </Card>
              <div style={{ marginTop: 8, fontSize: 11.5, color: 'var(--m-text-3)', padding: '0 2px' }}>
                {t('页面刷新由浏览器执行，只在页面打开时生效')}
              </div>
            </div>
          ) : null}
        </>
      ) : null}

      {editing ? (
        <Sheet open onOpenChange={open => { if (!open) setEditing(null) }}>
          <SheetContent>
            <SheetHeader className='flex-col items-start gap-1'>
              <SheetTitle>{editing.label || editing.id}</SheetTitle>
              <SheetDescription className='text-[12px] text-muted-foreground'>
                {t('可填 {min}–{max} {unit}', {
                  min: editing.min || 1,
                  max: editing.max || 9999,
                  unit: editing.unit === 'seconds' ? t('秒') : t('分钟'),
                })}
              </SheetDescription>
            </SheetHeader>
            <SheetBody>
              <Label htmlFor='task-interval'>{t('{name}间隔', { name: editing.label || editing.id || '' })}</Label>
              <InputGroup className='h-[46px]'>
                <InputGroupInput
                  id='task-interval'
                  value={draft}
                  onChange={event => setDraft(event.target.value.replace(/[^\d]/g, ''))}
                  inputMode='numeric'
                  className='font-mono text-base'
                />
                <InputGroupAddon align='inline-end'>
                  <InputGroupText>{editing.unit === 'seconds' ? t('秒') : t('分')}</InputGroupText>
                </InputGroupAddon>
              </InputGroup>
            </SheetBody>
            <SheetFooter>
              <Button size='touch' variant='outline' className='flex-1' onClick={() => setEditing(null)}>{t('取消')}</Button>
              <Button size='touch' className='flex-1' disabled={busy === editing.id} onClick={() => void saveInterval()}>{t('保存')}</Button>
            </SheetFooter>
          </SheetContent>
        </Sheet>
      ) : null}
    </div>
  )
}

function TaskRow({
  task,
  busy,
  onToggle,
  onRun,
  onEdit,
}: {
  task: ScheduledTask
  busy: boolean
  onToggle: (next: boolean) => void
  onRun?: () => void
  onEdit?: () => void
}) {
  const intervalText =
    task.interval && task.unit
      ? t('每 {n} {unit}', { n: task.interval, unit: task.unit === 'seconds' ? t('秒') : t('分钟') })
      : '—'
  const next = task.nextRunAt ? formatRelative(task.nextRunAt) : ''
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 11,
        padding: '13px 14px',
        borderBottom: '1px solid var(--m-line)',
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 13.5, fontWeight: 550 }}>{task.label}</div>
        <div style={{ fontSize: 11.5, color: 'var(--m-text-3)', marginTop: 3 }}>
          {intervalText}
          {task.lastRunAt ? ` · ${formatStamp(task.lastRunAt)}` : ''}
          {next ? ` · ${next}` : ''}
        </div>
        {task.lastError ? (
          <div style={{ fontSize: 11, color: 'var(--m-danger)', marginTop: 3 }}>{task.lastError}</div>
        ) : task.lastResult ? (
          <div style={{ fontSize: 11, color: 'var(--m-text-3)', marginTop: 3 }}>{task.lastResult}</div>
        ) : null}
        {task.canRun && onRun ? (
          <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
            <Button size='touch-sm' variant='outline'
              type='button'
              className='m-btn m-sm m-ghost'
              disabled={busy || task.running}
              onClick={onRun}
              style={{ height: 28, fontSize: 11.5, padding: '0 10px' }}
            >
              <Icon name='play' style={{ width: 12, height: 12 }} />
              {task.running ? t('执行中…') : t('立即执行')}
            </Button>
            {onEdit ? (
              <Button size='touch-sm' variant='outline'
                type='button'
                className='m-btn m-sm m-ghost'
                disabled={busy}
                onClick={onEdit}
                style={{ height: 28, fontSize: 11.5, padding: '0 10px' }}
              >
                <Icon name='edit' style={{ width: 12, height: 12 }} />
                {t('改间隔')}
              </Button>
            ) : null}
          </div>
        ) : null}
      </div>
      <SwitchSmall checked={Boolean(task.enabled)} disabled={busy} label={task.label || ''} onChange={onToggle} />
    </div>
  )
}

function SwitchSmall({
  checked,
  onChange,
  disabled,
  label,
}: {
  checked: boolean
  onChange: (next: boolean) => void
  disabled?: boolean
  label: string
}) {
  return (
    <UiSwitch size='touch' variant='success' checked={checked} aria-label={label} disabled={disabled} onCheckedChange={onChange} />
  )
}
