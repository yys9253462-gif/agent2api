import { t } from '../ui-kit/src/i18n'

export const API_ENDPOINTS = [
  { id: 'api-base', method: '', path: '/v1' },
  { id: 'api-chat', method: 'POST', path: '/v1/chat/completions' },
  { id: 'api-responses', method: 'POST', path: '/v1/responses' },
  { id: 'api-messages', method: 'POST', path: '/v1/messages' },
  { id: 'api-count-tokens', method: 'POST', path: '/v1/messages/count_tokens' },
  { id: 'api-image-generations', method: 'POST', path: '/v1/images/generations' },
  { id: 'api-image-edits', method: 'POST', path: '/v1/images/edits' },
  { id: 'api-image-files', method: 'GET', path: '/v1/images/files/{id}' },
  { id: 'api-models', method: 'GET', path: '/v1/models' },
  { id: 'api-health', method: 'GET', path: '/health' },
] as const

export function apiEndpoints() {
  const chat = t('对话协议与上下文预算')
  const images = t('图片生成与编辑')
  const probes = t('模型清单与健康检查')
  const labels: Record<typeof API_ENDPOINTS[number]['id'], { name: string; note: string; group: string }> = {
    'api-base': {
      name: 'Base URL', group: '',
      note: t('OpenAI 兼容客户端使用此地址；Anthropic 官方 SDK 使用下方说明中的网关根地址。'),
    },
    'api-chat': {
      name: 'OpenAI Chat Completions', group: chat,
      note: t('多数 OpenAI 兼容客户端的默认协议'),
    },
    'api-responses': {
      name: 'OpenAI Responses', group: chat,
      note: t('Codex / 新版 OpenAI SDK；无状态转发，历史由客户端携带。'),
    },
    'api-messages': {
      name: 'Anthropic Messages', group: chat,
      note: t('Claude Code / Anthropic SDK；模型与账号池和 OpenAI 入口共用。'),
    },
    'api-count-tokens': {
      name: t('Token 数量估算'), group: chat,
      note: t('本地上下文预算估算，返回 estimated=true；不是上游精确分词。'),
    },
    'api-image-generations': {
      name: t('图片生成'), group: images,
      note: t('文本生成图片；JSON 请求，非流式返回 base64 或下载链接。'),
    },
    'api-image-edits': {
      name: t('图片编辑'), group: images,
      note: t('参考图编辑；支持 multipart 文件上传或 JSON 图片引用。'),
    },
    'api-image-files': {
      name: t('临时图片下载'), group: images,
      note: t('仅用于响应返回的网关临时链接；{id} 必须替换为实际下载令牌。'),
    },
    'api-models': {
      name: t('模型列表'), group: probes,
      note: t('不强制鉴权；携带有效 Key 可查看该 Key 获准使用的模型。'),
    },
    'api-health': {
      name: t('健康检查'), group: probes,
      note: t('不强制鉴权；仅用于确认网关可访问，不保证模型或额度可用。'),
    },
  }
  return API_ENDPOINTS.map(endpoint => ({ ...endpoint, ...labels[endpoint.id] }))
}
