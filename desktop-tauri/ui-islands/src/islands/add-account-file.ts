import { t } from '../i18n'

export const ACCOUNT_FILE_FORMAT = 'agent2api-account'

export function isAccountExport(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value)
    && (value as Record<string, unknown>).format === ACCOUNT_FILE_FORMAT)
}

export function accountFilePayload(
  record: Record<string, unknown>,
  provider: string,
  name: string,
): Record<string, unknown> {
  if (record.provider !== provider) {
    throw new Error(t('账号文件的提供商与当前选择不一致，请选择对应的提供商后再导入'))
  }
  const payload: Record<string, unknown> = { ...record, provider }
  if (name) payload.name = name
  return payload
}

export function accountExportEdition(record: Record<string, unknown>): 'cn' | 'intl' | null {
  if (!isAccountExport(record)) return null
  if (record.provider === 'workbuddy') return 'cn'
  if (record.provider === 'workbuddy-intl') return 'intl'
  return null
}
