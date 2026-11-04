export function readFileAsDataUrl(file: File, errorMessage: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result || ''))
    reader.onerror = () => reject(new Error(errorMessage))
    reader.onabort = () => reject(new DOMException(errorMessage, 'AbortError'))
    reader.readAsDataURL(file)
  })
}
