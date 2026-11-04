import { useCallback, useEffect, useRef, useState } from 'react'
import { Button } from '@ui'
import { captchaPoolStats, verifyPlanChannel, type CaptchaPhase, type CaptchaPoolStats } from '../../api/zcode'
import { serverIdentity } from '../../lib/servers'
import { t } from '../../i18n'

export function ZcodeCaptchaPanel({ accountId, disabled, onBusy }: {
  accountId: string; disabled: boolean; onBusy?: (busy: boolean) => void
}) {
  const [stats, setStats] = useState<CaptchaPoolStats | null>(null)
  const [phase, setPhase] = useState<CaptchaPhase | ''>('')
  const [error, setError] = useState('')
  const controller = useRef<AbortController | null>(null)
  const mounted = useRef(true)
  const identity = serverIdentity()
  const refresh = useCallback(async (clearError = false) => {
    try {
      const value = await captchaPoolStats()
      if (mounted.current && serverIdentity() === identity) { setStats(value); if (clearError) setError('') }
    } catch (cause) {
      if (mounted.current && serverIdentity() === identity) setError(cause instanceof Error ? cause.message : String(cause))
    }
  }, [identity])
  useEffect(() => {
    mounted.current = true
    void refresh()
    return () => { mounted.current = false; controller.current?.abort(); onBusy?.(false) }
  }, [accountId, refresh, onBusy])

  async function verify() {
    if (controller.current || disabled) return
    const current = new AbortController()
    controller.current = current
    setError('')
    setPhase('preparing')
    onBusy?.(true)
    try {
      await verifyPlanChannel(accountId, current.signal, value => { if (mounted.current) setPhase(value) })
    } catch (cause) {
      if (mounted.current && !current.signal.aborted && (cause as Error)?.name !== 'CaptchaCancelledError') {
        setError(cause instanceof Error ? cause.message : String(cause))
      }
    } finally {
      if (controller.current === current) controller.current = null
      if (mounted.current && serverIdentity() === identity) { setPhase(''); onBusy?.(false); void refresh() }
    }
  }

  const failure = error || stats?.error || stats?.configError
  const text = stats?.required === false ? t('当前模型请求免人机验证')
    : Number(stats?.ready) > 0 ? t('验证码令牌：{counts}', { counts: `${stats?.ready} / ${stats?.target || 3}` })
      : stats?.needsTokens ? t('套餐池请求需要验证，请主动开始验证后重试请求')
        : t('没有近期套餐池请求，不生成验证码令牌')
  return (
    <section className='m-ck-account'>
      <div className='m-ck-title'>{t('套餐池人机验证')}</div>
      <p className={failure ? 'm-ck-error' : 'm-ck-note'}>{failure || (stats ? text : t('读取中…'))}</p>
      <p className='m-ck-note'>{t('后台只读取状态，不会自动弹出验证码；验证结果一次性使用，短时间内过期。')}</p>
      <div className='m-ck-actions'>
        {stats?.required !== false && Number(stats?.startPlanAccounts) > 0 ? (
          <Button size='sm' variant='outline' disabled={disabled || Boolean(phase) || Boolean(stats?.error || stats?.configError)} onClick={() => void verify()}>
            {phase ? t('正在验证…') : t('开始验证')}
          </Button>
        ) : null}
        {phase ? <Button size='sm' variant='outline' disabled={phase === 'submitting'} onClick={() => controller.current?.abort()}>{t('取消验证')}</Button> : null}
        <Button size='sm' variant='ghost' disabled={disabled || Boolean(phase)} onClick={() => void refresh(true)}>{t('刷新')}</Button>
      </div>
    </section>
  )
}
