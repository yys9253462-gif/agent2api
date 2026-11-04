import { t } from '../i18n'
import { bridgeApi, errorMessage, loadCenter, patch, store, toast, type OnboardingCache } from './checkin-store'
import { normalizeOnboardingTasks } from './checkin-onboarding-cache'

export function onboardingOf(id: string): OnboardingCache {
  return store.onboarding.get(id) ?? {
    status: 'idle', tasks: [], earned: 0, claiming: false, settled: false,
  }
}

function patchOnboarding(id: string, next: Partial<OnboardingCache>): void {
  const map = new Map(store.onboarding)
  map.set(id, { ...onboardingOf(id), ...next })
  patch({ onboarding: map })
}

/** 手动查询可强制刷新；批量查询跳过已结清的账号。 */
export async function queryOnboarding(id: string, refresh = false): Promise<void> {
  const cached = onboardingOf(id)
  if (cached.status === 'loading' || cached.claiming) return
  if (!refresh && cached.settled) return
  patchOnboarding(id, { status: 'loading', error: '' })
  try {
    const data = await bridgeApi().getOnboardingTasks?.(id, refresh)
    if (!data) throw new Error(t('后端未返回新手福利数据'))
    patchOnboarding(id, {
      status: 'loaded',
      tasks: normalizeOnboardingTasks(data.tasks),
      earned: Number(data.earned) || 0,
      settled: data.settled === true,
      settledAt: Number(data.settledAt) || undefined,
      checkedAt: Date.now(),
    })
    await loadCenter()
  } catch (error) {
    patchOnboarding(id, { status: 'error', error: errorMessage(error) })
  }
}

export async function claimOnboarding(id: string): Promise<void> {
  const cached = onboardingOf(id)
  if (cached.status !== 'loaded' || cached.claiming || cached.settled) return
  patchOnboarding(id, { claiming: true, error: '' })
  try {
    const data = await bridgeApi().claimOnboardingTasks?.(id)
    if (!data) throw new Error(t('后端未返回新手福利数据'))
    const failed = Number(data.failed) || 0
    patchOnboarding(id, {
      claiming: false,
      status: 'loaded',
      tasks: Array.isArray(data.tasks) ? normalizeOnboardingTasks(data.tasks) : cached.tasks,
      earned: Number.isFinite(data.earned) ? Number(data.earned) : cached.earned,
      settled: data.settled === true,
      settledAt: Number(data.settledAt) || undefined,
      checkedAt: Date.now(),
    })
    await loadCenter()
    const message = typeof data.message === 'string' ? data.message.trim() : ''
    toast(message || (failed > 0
      ? t('新手福利领取失败 {n} 项', { n: failed })
      : t('✅ 新手福利领取完成')), failed > 0 ? 'err' : 'ok')
  } catch (error) {
    patchOnboarding(id, { claiming: false })
    toast(t('领取失败：{error}', { error: errorMessage(error) }), 'err')
  }
}
