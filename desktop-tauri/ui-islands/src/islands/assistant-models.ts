import { t } from '../i18n'

export { modelOptionsFrom, selectedModel, type ModelOption } from '../../../ui-shared/assistant-models'

export function unavailableModelMessage(name: string): string {
  return t('模型「{model}」当前不在可用列表中，请重新选择；也可到「模型管理」或「插件」页检查是否已启用。', { model: name })
}
