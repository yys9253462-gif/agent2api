/** 账号设置、分组设置与批量操作弹窗的状态宿主。 */

import { useSyncExternalStore } from 'react'
import { closeDialog, getStore, subscribe } from './accounts-data'
import { BatchDialog } from './accounts-batch-dialog'
import { GroupSettingsDialog } from './accounts-group-settings'
import { AccountSettingsDialog } from './accounts-settings-dialog'

export { AccountSettingsDialog } from './accounts-settings-dialog'
export { draftOfProxy, ProxyForm, readProxyDraft } from './accounts-proxy-form'
export type { ProxyDraft, ProxyPayload } from './accounts-proxy-form'

export function AccountsDialogs() {
  const store = useSyncExternalStore(subscribe, () => getStore())
  if (!store.dialog) return null
  if (store.dialog.kind === 'settings') {
    return <AccountSettingsDialog key={store.dialog.id} id={store.dialog.id} onClose={closeDialog} />
  }
  if (store.dialog.kind === 'group') {
    return <GroupSettingsDialog key={store.dialog.provider} provider={store.dialog.provider} onClose={closeDialog} />
  }
  return <BatchDialog key={store.dialog.ids.join(',')} ids={store.dialog.ids} action={store.dialog.action}
    onClose={closeDialog} />
}
