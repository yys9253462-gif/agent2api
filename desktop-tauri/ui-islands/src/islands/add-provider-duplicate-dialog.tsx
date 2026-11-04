import * as React from 'react'
import { Button, Dialog, DialogBody, DialogContent, DialogFooter, DialogHeader, DialogTitle, Item, ItemContent, ItemDescription, ItemTitle } from '@ui'

import type { CustomProviderRecord } from './add-account-bridge'
import { t } from '../i18n'

export function DuplicateProviderDialog({
  providers,
  onCreate,
  onExisting,
  onClose,
}: {
  providers: CustomProviderRecord[]
  onCreate: () => void
  onExisting: (provider: CustomProviderRecord) => void
  onClose: () => void
}): React.ReactElement {
  return (
    <Dialog open onOpenChange={open => { if (!open) onClose() }}>
      <DialogContent overlayForceRender className='w-[min(560px,calc(100vw-48px))]'>
        <DialogHeader>
          <DialogTitle>{t('该地址已有提供商')}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <p className='mb-3 text-sm text-subtle'>{t('相同协议和地址的提供商已经存在。可以继续新建独立提供商，或明确选择已有提供商添加账号。')}</p>
          <div className='space-y-2'>
            {providers.map(provider => (
              <Item key={provider.id} variant='outline'>
                <ItemContent>
                  <ItemTitle>{provider.name || provider.id}</ItemTitle>
                  <ItemDescription>{provider.baseUrl}</ItemDescription>
                </ItemContent>
                <Button variant='outline' onClick={() => onExisting(provider)}>
                  {t('添加到此提供商')}
                </Button>
              </Item>
            ))}
          </div>
        </DialogBody>
        <DialogFooter>
          <Button variant='outline' onClick={onClose}>{t('取消')}</Button>
          <Button onClick={onCreate}>{t('继续新建独立提供商')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
