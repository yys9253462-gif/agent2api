import { apiCall } from '../api/client'
import { t } from '../i18n'
import { createAssistantImageTools } from '../../../ui-shared/assistant-image-tools'

export const ASSISTANT_IMAGE_TOOLS = createAssistantImageTools(
  (path, body, signal, apiKey) => apiCall(path, { method: 'POST', body, signal, apiKey }), t,
)
