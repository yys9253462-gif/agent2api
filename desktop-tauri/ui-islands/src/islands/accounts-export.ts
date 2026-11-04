import { t } from '../i18n'
import { errorMessage, shared, toast, type AccountExportSelection } from './accounts-shared'

let exporting = false

export async function exportAccountFiles(selection: AccountExportSelection): Promise<void> {
  if (exporting) {
    toast(t('账号正在导出，请稍候'))
    return
  }
  const bridge = shared().workbuddyDesktop
  if (!bridge?.exportAccountFiles) {
    toast(t('当前客户端不支持逐账号导出，请更新客户端'), 'err')
    return
  }
  exporting = true
  try {
    const confirm = shared().wbConfirm?.ask
    const html = t('导出文件包含登录凭证，可直接用于登录。请妥善保管，不要外传或上传到公共位置。')
      + (selection.provider ? `<br><br>${t('本组每个账号会分别保存为一个 JSON 文件，包含被筛选隐藏的账号。')}` : '')
      + (selection.provider && bridge.platform === 'web'
        ? `<br><br>${t('网页端会逐个下载文件；浏览器提示时请允许本网站下载多个文件。')}` : '')
    const accepted = confirm
      ? await confirm({
          title: selection.provider ? t('导出本组账号') : t('导出账号'),
          html,
          okText: t('导出'),
        })
      : window.confirm(t('导出文件包含登录凭证，可直接用于登录。请妥善保管，不要外传或上传到公共位置。'))
    if (!accepted) return
    const result = await bridge.exportAccountFiles(selection)
    if (result?.canceled) return
    if (!result) throw new Error(t('导出未返回结果'))
    const count = result.count || 0
    const failed = result.failed || []
    if (failed.length) {
      const first = failed[0]
      const detail = [first.name || first.id, first.error].filter(Boolean).join('：')
      toast(t('已导出 {count} 个账号文件，{failed} 个未导出：{reason}', {
        count, failed: failed.length, reason: detail,
      }), 'err')
    } else if (count) {
      toast(result.downloadStarted
        ? t('已发起 {count} 个账号文件的下载，请在浏览器下载列表确认', { count })
        : t('✅ 已导出 {count} 个账号文件', { count }), 'ok')
    } else {
      toast(t('没有可导出的账号'))
    }
  } catch (error) {
    toast(t('账号导出失败：{reason}', { reason: errorMessage(error) }), 'err')
  } finally {
    exporting = false
  }
}
