export type ApiExample = { label: string; language: 'bash' | 'json' | 'python'; code: string }

const json = (body: unknown) => JSON.stringify(body, null, 2)

export function apiExampleCode(origin: string) {
  const base = origin.replace(/\/+$/, '')
  const post = (path: string, body: unknown, streaming = false, anthropic = false) => [
    `curl${streaming ? ' -N' : ''} -X POST "${base}${path}" \\`,
    anthropic ? '  -H "x-api-key: YOUR_GATEWAY_KEY" \\' : '  -H "Authorization: Bearer YOUR_GATEWAY_KEY" \\',
    ...(anthropic ? ['  -H "anthropic-version: 2023-06-01" \\'] : []),
    '  -H "Content-Type: application/json" \\',
    `  -d '${json(body)}'`,
  ].join('\n')
  const chat = {
    model: 'CHAT_MODEL_ID',
    messages: [{ role: 'system', content: 'You are a helpful assistant.' }, { role: 'user', content: 'Hello!' }],
    stream: false,
  }
  const message = {
    model: 'CHAT_MODEL_ID', max_tokens: 1024,
    system: 'You are a helpful assistant.',
    messages: [{ role: 'user', content: 'Hello!' }], stream: false,
  }
  const generation = {
    model: 'IMAGE_MODEL_ID', prompt: 'A small raccoon reading a book, flat illustration.',
    n: 1, size: '1024x1024', response_format: 'b64_json', output_format: 'png',
  }
  return {
    setup: `export GATEWAY_URL="${base}"
export GATEWAY_KEY="YOUR_GATEWAY_KEY"
export CHAT_MODEL_ID="CHAT_MODEL_ID"
export IMAGE_MODEL_ID="IMAGE_MODEL_ID"`,
    models: `curl "${base}/v1/models" \\
  -H "Authorization: Bearer YOUR_GATEWAY_KEY"`,
    health: `curl "${base}/health"`,
    modelResponse: json({ object: 'list', data: [
      { id: 'CHAT_MODEL_ID', object: 'model', owned_by: 'PROVIDER_ID', kind: 'chat',
        supports_tool_call: true, supports_images: true, supports_reasoning: true,
        supports_image_generation: false, supports_image_editing: false, input_modalities: ['text', 'image'] },
      { id: 'IMAGE_MODEL_ID', object: 'model', owned_by: 'PROVIDER_ID', kind: 'image',
        supports_images: true, supports_image_generation: true, supports_image_editing: true,
        input_modalities: ['text', 'image'], output_modalities: ['image'] },
    ] }),
    chat: post('/v1/chat/completions', chat),
    chatStream: post('/v1/chat/completions', { ...chat, stream: true, stream_options: { include_usage: true } }, true),
    chatResponse: json({ id: 'chatcmpl-EXAMPLE', object: 'chat.completion', model: 'CHAT_MODEL_ID',
      choices: [{ index: 0, message: { role: 'assistant', content: 'Hello!' }, finish_reason: 'stop' }],
      usage: { prompt_tokens: 12, completion_tokens: 4, total_tokens: 16 } }),
    vision: post('/v1/chat/completions', { model: 'CHAT_MODEL_ID', messages: [{ role: 'user', content: [
      { type: 'text', text: 'Describe this image.' },
      { type: 'image_url', image_url: { url: 'https://YOUR_IMAGE_HOST/reference.png' } },
    ] }], stream: false }),
    tools: json({ model: 'CHAT_MODEL_ID', messages: [{ role: 'user', content: 'What is the weather in Beijing?' }],
      tools: [{ type: 'function', function: { name: 'get_weather', description: 'Get the weather for a city.',
        parameters: { type: 'object', properties: { city: { type: 'string' } }, required: ['city'], additionalProperties: false } } }],
      tool_choice: 'auto', stream: false }),
    toolResult: json({ model: 'CHAT_MODEL_ID', messages: [
      { role: 'user', content: 'What is the weather in Beijing?' },
      { role: 'assistant', content: null, tool_calls: [{ id: 'call_EXAMPLE', type: 'function',
        function: { name: 'get_weather', arguments: '{"city":"Beijing"}' } }] },
      { role: 'tool', tool_call_id: 'call_EXAMPLE', content: '{"temperature_c":22}' },
    ], stream: false }),
    responses: post('/v1/responses', { model: 'CHAT_MODEL_ID', instructions: 'You are a helpful assistant.',
      input: 'Hello!', max_output_tokens: 1024, store: false, stream: false }),
    responsesResponse: json({ id: 'resp_EXAMPLE', object: 'response', status: 'completed', model: 'CHAT_MODEL_ID',
      output: [{ type: 'message', role: 'assistant', content: [{ type: 'output_text', text: 'Hello!' }] }],
      output_text: 'Hello!', store: false, usage: { input_tokens: 12, output_tokens: 4, total_tokens: 16 } }),
    messages: post('/v1/messages', message, false, true),
    messagesResponse: json({ id: 'msg_EXAMPLE', type: 'message', role: 'assistant', model: 'CHAT_MODEL_ID',
      content: [{ type: 'text', text: 'Hello!' }], stop_reason: 'end_turn',
      usage: { input_tokens: 12, output_tokens: 4 } }),
    countTokens: post('/v1/messages/count_tokens', { model: message.model, system: message.system, messages: message.messages }, false, true),
    countResponse: json({ input_tokens: 42, estimated: true }),
    generation: post('/v1/images/generations', generation),
    editMultipart: `curl -X POST "${base}/v1/images/edits" \\
  -H "Authorization: Bearer YOUR_GATEWAY_KEY" \\
  -F "model=IMAGE_MODEL_ID" \\
  -F "image=@reference.png" \\
  --form-string "prompt=Replace the background with a sunny garden; keep the subject unchanged." \\
  -F "n=1" \\
  -F "size=auto" \\
  -F "response_format=b64_json" \\
  -F "output_format=png"`,
    editMultiple: `curl -X POST "${base}/v1/images/edits" \\
  -H "Authorization: Bearer YOUR_GATEWAY_KEY" \\
  -F "model=IMAGE_MODEL_ID" \\
  -F "image[]=@subject.png" \\
  -F "image[]=@style.png" \\
  --form-string "prompt=Redraw the subject in image 1 using the visual style of image 2." \\
  -F "n=1" \\
  -F "response_format=b64_json"`,
    editJson: post('/v1/images/edits', { model: 'IMAGE_MODEL_ID',
      prompt: 'Replace the background with a sunny garden; keep the subject unchanged.',
      images: [{ image_url: 'https://YOUR_IMAGE_HOST/reference.png' }],
      n: 1, size: 'auto', response_format: 'b64_json', output_format: 'png' }),
    imageBase64: json({ created: 1700000000, data: [{ b64_json: 'BASE64_IMAGE_DATA' }], output_format: 'png' }),
    imageUrl: json({ created: 1700000000, data: [{ url: `${base}/v1/images/files/IMAGE_DOWNLOAD_TOKEN` }], output_format: 'png' }),
    download: `curl -L "${base}/v1/images/files/IMAGE_DOWNLOAD_TOKEN" \\
  -o generated.png`,
    imageSdk: `import base64
import os
from pathlib import Path
from openai import OpenAI

client = OpenAI(
    base_url="${base}/v1",
    api_key=os.environ["GATEWAY_KEY"],
    max_retries=0,
    timeout=300.0,
)
result = client.images.generate(
    model=os.environ["IMAGE_MODEL_ID"],
    prompt="A small raccoon reading a book, flat illustration.",
    n=1,
    response_format="b64_json",
    output_format="png",
)
Path("generated.png").write_bytes(base64.b64decode(result.data[0].b64_json))

with open("reference.png", "rb") as image:
    edited = client.images.edit(
        model=os.environ["IMAGE_MODEL_ID"],
        image=image,
        prompt="Replace the background with a sunny garden.",
        n=1,
        response_format="b64_json",
        output_format="png",
    )
Path("edited.png").write_bytes(base64.b64decode(edited.data[0].b64_json))`,
    error: json({ error: { message: "Model 'UNKNOWN_MODEL' is not available.", type: 'invalid_request_error', code: 'model_not_found' } }),
    anthropicError: json({ type: 'error', error: { type: 'invalid_request_error', message: 'Invalid request.' } }),
  }
}
