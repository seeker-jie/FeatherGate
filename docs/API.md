# FeatherGate API 文档

## 概述

FeatherGate 提供完全兼容 OpenAI 的 REST API。所有端点都接受和返回 OpenAI 格式的数据。

## 基础信息

- **基础 URL**: `http://localhost:8080`
- **内容类型**: `application/json`
- **认证**: 通过配置文件管理 API 密钥，客户端无需提供

## 端点列表

### 1. 聊天完成

创建聊天完成请求。

**端点**: `POST /v1/chat/completions`

**请求体**:

```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 100,
  "top_p": 1.0,
  "stream": false
}
```

**参数说明**:

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| model | string | 是 | 模型名称（配置文件中的 model_name） |
| messages | array | 是 | 消息列表 |
| temperature | number | 否 | 采样温度 (0-2)，默认 1.0 |
| max_tokens | integer | 否 | 最大生成 token 数 |
| top_p | number | 否 | 核采样参数 (0-1)，默认 1.0 |
| stream | boolean | 否 | 是否流式返回，默认 false |

**响应（非流式）**:

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 9,
    "total_tokens": 19
  }
}
```

**响应（流式）**:

流式响应使用 Server-Sent Events (SSE) 格式：

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

**错误响应**:

```json
{
  "error": {
    "message": "Model not found: invalid-model",
    "type": "feathergate_error"
  }
}
```

### 2. 列出模型

获取所有可用模型列表。

**端点**: `GET /v1/models`

**响应**:

```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1677652288,
      "owned_by": "feathergate"
    },
    {
      "id": "claude-opus",
      "object": "model",
      "created": 1677652288,
      "owned_by": "feathergate"
    }
  ]
}
```

### 3. 健康检查

检查服务健康状态。

**端点**: `GET /health`

**响应**:

```json
{
  "status": "ok",
  "service": "feathergate"
}
```

### 4. Prometheus 指标

获取 Prometheus 格式的监控指标。

**端点**: `GET /metrics`

**响应**:

```
# HELP feathergate_requests_total Total number of requests
# TYPE feathergate_requests_total counter
feathergate_requests_total 1234

# HELP feathergate_requests_successful Number of successful requests
# TYPE feathergate_requests_successful counter
feathergate_requests_successful 1200

# HELP feathergate_requests_failed Number of failed requests
# TYPE feathergate_requests_failed counter
feathergate_requests_failed 34
```

## 流式支持状态

| 提供商 | 非流式 | 流式 | 状态 |
|--------|--------|------|------|
| OpenAI | ✅ | ✅ | 完全支持 |
| Anthropic | ✅ | 🚧 | 协议转换已完成，流式进行中 |
| Gemini | ✅ | 🚧 | 协议转换已完成，流式进行中 |

## 错误码

| HTTP 状态码 | 说明 |
|------------|------|
| 200 | 成功 |
| 400 | 请求参数错误或不支持的提供商 |
| 404 | 模型未找到 |
| 500 | 内部服务器错误 |
| 502 | 上游 API 错误 |

## 使用示例

### Python (OpenAI SDK)

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="dummy"  # FeatherGate 不需要客户端提供 API key
)

# 非流式
response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "Hello!"}
    ]
)
print(response.choices[0].message.content)

# 流式
stream = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello!"}],
    stream=True
)
for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

### cURL

```bash
# 非流式
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# 流式
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'
```

### JavaScript (fetch)

```javascript
// 非流式
const response = await fetch('http://localhost:8080/v1/chat/completions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    model: 'gpt-4',
    messages: [{ role: 'user', content: 'Hello!' }]
  })
});
const data = await response.json();
console.log(data.choices[0].message.content);

// 流式
const response = await fetch('http://localhost:8080/v1/chat/completions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    model: 'gpt-4',
    messages: [{ role: 'user', content: 'Hello!' }],
    stream: true
  })
});

const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;

  const chunk = decoder.decode(value);
  const lines = chunk.split('\n');

  for (const line of lines) {
    if (line.startsWith('data: ')) {
      const data = line.slice(6);
      if (data === '[DONE]') break;
      const json = JSON.parse(data);
      const content = json.choices[0]?.delta?.content;
      if (content) process.stdout.write(content);
    }
  }
}
```

## 注意事项

1. **流式支持限制**: 当前仅 OpenAI 提供商支持流式响应。Anthropic 和 Gemini 的流式支持正在开发中。

2. **API 密钥管理**: FeatherGate 在配置文件中管理所有上游 API 密钥，客户端无需提供。

3. **模型名称**: 客户端使用配置文件中的 `model_name`，而非上游提供商的实际模型 ID。

4. **协议转换**: FeatherGate 自动处理不同提供商的协议差异，客户端始终使用 OpenAI 格式。
