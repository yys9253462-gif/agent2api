import type { CheckinResultRow } from './checkin'

export function claimCompleted(claim: CheckinResultRow['claim']): boolean {
  if (!claim || claim.error?.trim()) return false
  if (claim.outcome !== undefined) return claim.outcome === 'claimed' || claim.outcome === 'done'
  if (claim.success === true || claim.alreadyCompleted === true) return true
  const message = claim.msg ?? ''
  return !/可能|未能|未见|失败/.test(message) && /已签到|已领取/.test(message)
}

export function rowCompleted(row: CheckinResultRow): boolean {
  if (row.error?.trim()) return false
  const active = row.activity?.report?.succeeded === true || row.activity?.webConversation?.succeeded === true
  return row.mode === 'keepalive' ? active || row.activity?.pokeSucceeded === true : claimCompleted(row.claim) || active
}

export function rowPending(row: CheckinResultRow): boolean {
  if (row.error?.trim() || row.claim?.error?.trim() || rowCompleted(row)) return false
  const claim = row.claim
  if (claim?.outcome !== undefined) return claim.outcome === 'inactive' || claim.outcome === 'ineligible'
  return /没有可领取的签到活动|未下发每日积分活动|暂未给该账号下发每日积分活动|暂无可领取活动|等待重试/.test(claim?.msg ?? '')
}

export function rowMessage(row: CheckinResultRow): string {
  return row.error?.trim() || row.claim?.error?.trim() || row.claim?.msg?.trim() || row.activity?.error?.trim() || ''
}
