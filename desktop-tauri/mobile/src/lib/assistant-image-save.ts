import { Capacitor, registerPlugin } from '@capacitor/core'
import { saveAssistantImageFile, type ImageFileSaver } from '../../../ui-shared/assistant-image-save'
import { t } from '../i18n'

type ImageSaverPlugin = {
  save(options: { data: string; name: string; mime: string }): Promise<{ saved: boolean }>
}

const ImageSaver = registerPlugin<ImageSaverPlugin>('ImageSaver')

export const saveMobileAssistantImage: ImageFileSaver = async (image, blob) => {
  if (!Capacitor.isNativePlatform()) { await saveAssistantImageFile(image, blob); return }
  if (!Capacitor.isPluginAvailable('ImageSaver')) throw new Error(t('当前安装包未包含原生图片保存能力，请更新应用。'))
  const data = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(new Error(t('无法读取图片')))
    reader.onload = () => resolve(String(reader.result).split(',')[1] || '')
    reader.readAsDataURL(blob)
  })
  const extension = image.mime === 'image/jpeg' ? 'jpg' : image.mime === 'image/webp' ? 'webp' : 'png'
  await ImageSaver.save({ data, name: `image-${image.id.slice(0, 8)}.${extension}`, mime: image.mime })
}
