import { useEffect, useMemo, useState } from 'react'
import { Alert, Button, Empty, InputGroup, InputGroupAddon, InputGroupInput, Sheet, SheetBody, SheetContent, SheetHeader, SheetTitle } from '@ui'
import { selectedModel, type ModelOption } from '../../../ui-shared/assistant-models'
import { Icon } from './Icon'
import { t } from '../i18n'

type Props = {
  open: boolean
  onClose: () => void
  models: ModelOption[]
  current: string
  defaultModel: string
  followingDefault: boolean
  ready: boolean
  loading: boolean
  warning: string
  onRefresh: () => void
  onPick: (value: string) => void
}

export function AssistantModelPicker({
  open, onClose, models, current, defaultModel, followingDefault, ready, loading, warning, onRefresh, onPick,
}: Props) {
  const [query, setQuery] = useState('')
  useEffect(() => { if (!open) setQuery('') }, [open])
  const filtered = useMemo(() => {
    const keyword = query.trim().toLowerCase()
    return keyword ? models.filter(item => item.name.toLowerCase().includes(keyword)) : models
  }, [models, query])
  const currentOption = selectedModel(models, current)
  const defaultAvailable = Boolean(selectedModel(models, defaultModel))

  return (
    <Sheet open={open} onOpenChange={next => { if (!next) onClose() }}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{t('选择对话模型')}</SheetTitle>
          <Button size='icon-sm' variant='ghost' disabled={loading} onClick={onRefresh}
            title={t('刷新列表')} aria-label={t('刷新列表')}><Icon name='refresh' /></Button>
        </SheetHeader>
        <SheetBody>
          <InputGroup>
            <InputGroupInput value={query} onChange={event => setQuery(event.currentTarget.value)}
              placeholder={t('搜索模型…')} aria-label={t('搜索模型…')} />
            <InputGroupAddon><Icon name='search' style={{ width: 16, height: 16 }} /></InputGroupAddon>
          </InputGroup>
          {warning && !loading ? <Alert variant='destructive' role='status'>{warning}</Alert> : null}
          <Button size='default' variant='ghost' type='button' aria-pressed={followingDefault}
            disabled={!ready || !defaultAvailable}
            className='m-row h-auto min-h-[52px] w-full justify-start rounded-none border-0 px-3.5 py-3 text-left'
            onClick={() => onPick('')}
            style={{
              borderRadius: 'var(--m-r-md)',
              border: '1px solid var(--m-line)',
              background: followingDefault ? 'var(--m-primary-soft)' : 'var(--m-surface)',
            }}
          >
            <span className='m-logo'><Icon name='settings' /></span>
            <div className='m-grow'>
              <div className='m-t1'>{t('跟随网关设置')}</div>
              <div className='m-t2 m-mono' style={{ fontSize: 11.5 }}>
                {defaultModel || t('还没有配置默认模型')}
                {defaultModel && ready && !defaultAvailable ? ` · ${t('不可用')}` : ''}
              </div>
            </div>
            {followingDefault ? <Icon name='check' style={{ width: 16, height: 16, color: 'var(--m-primary)' }} /> : null}
          </Button>
          {filtered.length ? filtered.map(item => {
            const picked = !followingDefault && currentOption?.name === item.name
            return (
              <Button size='default' variant='ghost' key={item.name} type='button' aria-pressed={picked}
                disabled={!ready}
                className='m-row h-auto min-h-[44px] w-full justify-start rounded-none border-0 px-3.5 py-2.5 text-left'
                onClick={() => onPick(item.name)}
                style={{
                  borderRadius: 'var(--m-r-md)',
                  border: '1px solid var(--m-line)',
                  background: picked ? 'var(--m-primary-soft)' : 'var(--m-surface)',
                }}
              >
                <span className='m-grow m-t1 m-mono min-w-0 truncate text-[12.5px]' title={item.name}>{item.name}</span>
                {picked ? <Icon name='check' style={{ width: 16, height: 16, color: 'var(--m-primary)' }} /> : null}
              </Button>
            )
          }) : <Empty>
            {loading ? t('正在读取可用模型…') : query.trim() ? t('没有匹配的模型') : t('没有可用的模型')}
          </Empty>}
          <p className='text-[11.5px] leading-[1.7] text-subtle'>
            {t('这里只改本次会话用的模型，不会改网关里保存的默认值。')}
          </p>
        </SheetBody>
      </SheetContent>
    </Sheet>
  )
}
