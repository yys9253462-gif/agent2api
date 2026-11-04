import type { ImageAsset } from './assistant-image-tools'

type SaveBridge = { saveAssistantImage?: (id: string) => Promise<unknown> }
export type ImageFileSaver = (image: ImageAsset, blob: Blob) => Promise<void>

export async function saveAssistantImageFile(image: ImageAsset, blob: Blob): Promise<void> {
  const bridge = (window as unknown as { workbuddyDesktop?: SaveBridge }).workbuddyDesktop
  if (bridge?.saveAssistantImage) { await bridge.saveAssistantImage(image.id); return }
  const extension = image.mime === 'image/jpeg' ? 'jpg' : image.mime === 'image/webp' ? 'webp' : 'png'
  const name = `image-${image.id.slice(0, 8)}.${extension}`
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = name
  document.body.append(link)
  link.click()
  link.remove()
  setTimeout(() => URL.revokeObjectURL(url), 60_000)
}
