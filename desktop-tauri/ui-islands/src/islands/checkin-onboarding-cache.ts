import type { OnboardingCache } from './checkin-store'
import type { CheckinCenter, OnboardingTask } from './checkin-types'

export function normalizeOnboardingTasks(raw: unknown): OnboardingTask[] {
  if (!Array.isArray(raw)) return []
  return raw.flatMap(item => {
    const row = (item ?? {}) as Record<string, unknown>
    const key = typeof row.key === 'string' ? row.key : ''
    if (!key) return []
    return [{
      key,
      title: typeof row.title === 'string' && row.title ? row.title : key,
      points: Number(row.points) || 0,
      done: row.done === true,
      blocked: row.blocked === true,
      claiming: row.claiming === true,
      error: typeof row.error === 'string' ? row.error : '',
    }]
  })
}

export function onboardingFromCenter(
  center: CheckinCenter,
  previous: Map<string, OnboardingCache>,
): Map<string, OnboardingCache> {
  const next = new Map<string, OnboardingCache>()
  for (const row of center.extras.onboarding) {
    const cached = previous.get(row.id)
    if (cached?.status === 'loading' || cached?.claiming) {
      next.set(row.id, cached)
      continue
    }
    if (row.settled) {
      next.set(row.id, {
        status: 'loaded',
        tasks: normalizeOnboardingTasks(row.settled.tasks),
        earned: Number(row.settled.earned) || 0,
        claiming: false,
        settled: true,
        settledAt: Number(row.settled.at) || undefined,
      })
    } else if (cached && !cached.settled) {
      next.set(row.id, cached)
    }
  }
  return next
}
