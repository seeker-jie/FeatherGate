# FeatherGate 架构文档

## 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                        Client Layer                          │
│  (OpenAI SDK, cURL, 任何 OpenAI 兼容客户端)                  │
└────────────────────────┬────────────────────────────────────┘
                         │ HTTP/JSON (OpenAI 格式)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    FeatherGate Proxy                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              HTTP Server (Hyper + Tokio)             │   │
│  └────────────────────────┬─────────────────────────────┘   │
│                           │                                  │
│  ┌────────────────────────▼─────────────────────────────┐   │
│  │                    Router Layer                      │   │
│  │  • 解析 model_name                                   │   │
│  │  • 查找配置                                          │   │
│  │  • 确定 provider                                     │   │
│  └────────────────────────┬─────────────────────────────┘   │
│                           │                                  │
│         ┌─────────────────┼─────────────────┐               │
│         ▼                 ▼                 ▼               │
│  ┌──────────┐      ┌──────────┐      ┌──────────┐          │
│  │  OpenAI  │      │Anthropic │      │  Gemini  │          │
│  │ Provider │      │ Provider │      │ Provider │          │
│  └──────────┘      └──────────┘      └──────────┘          │
└─────────┬───────────────┬───────────────┬──────────────────┘
          │               │               │
          ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Upstream LLM APIs                         │
│     OpenAI API      Anthropic API      Gemini API           │
└─────────────────────────────────────────────────────────────┘
```

## 核心模块

### 1. HTTP Server (`src/server/`)

基于 Hyper 1.0 和 Tokio 的异步 HTTP 服务器。

**职责**:
- 接收客户端 HTTP 请求
- 路由到对应的处理函数
- 返回 HTTP 响应

**关键文件**:
- `mod.rs`: 服务器主循环
- `handlers.rs`: 请求处理函数
- `streaming.rs`: SSE 流式响应格式化

### 2. Router (`src/providers/routing.rs`)

请求路由和提供商选择。

**职责**:
- 根据 `model_name` 查找配置
- 解析 `provider/model-id` 格式
- 路由到对应的 provider 实现

**路由逻辑**:
```rust
model_name (客户端请求)
  → 查找 ModelConfig
  → 解析 provider
  → 调用对应 provider 的 forward_request()
```

### 3. Provider Clients (`src/providers/`)

各个 LLM 提供商的客户端实现。

**统一接口**:
```rust
pub async fn forward_request(
    config: &ModelConfig,
    req: ChatRequest,
) -> Result<ChatResponse>
```

**提供商实现**:
- `openai.rs`: OpenAI 直接透传
- `anthropic.rs`: Anthropic Claude 协议转换
- `gemini.rs`: Google Gemini 协议转换

### 4. Configuration (`src/config/`)

配置文件解析和管理。

**职责**:
- 加载 YAML 配置
- 环境变量替换
- 配置验证

### 5. Types (`src/types/`)

OpenAI 兼容的类型定义。

**核心类型**:
- `ChatRequest`: 聊天请求
- `ChatResponse`: 聊天响应
- `Message`: 消息
- `ChatStreamChunk`: 流式响应块

### 6. Error Handling (`src/error.rs`)

统一错误处理。

**错误类型**:
- `ConfigError`: 配置错误
- `ModelNotFound`: 模型未找到
- `UnsupportedProvider`: 不支持的提供商
- `UpstreamError`: 上游 API 错误

### 7. Metrics (`src/metrics/`)

Prometheus 指标收集。

**指标**:
- `total_requests`: 总请求数
- `successful_requests`: 成功请求数
- `failed_requests`: 失败请求数

## 数据流

### 非流式请求

```
1. 客户端发送 POST /v1/chat/completions
   ↓
2. handlers::chat_completions() 解析请求
   ↓
3. routing::route_request() 路由
   ↓
4. provider::forward_request() 转发
   ↓
5. 协议转换 (如需要)
   ↓
6. HTTP 请求到上游 API
   ↓
7. 接收上游响应
   ↓
8. 协议转换回 OpenAI 格式
   ↓
9. 返回给客户端
```

### 流式请求

```
1. 客户端发送 POST /v1/chat/completions (stream=true)
   ↓
2. handlers::chat_completions_stream() 检测流式
   ↓
3. routing::route_request_stream() 路由
   ↓
4. provider::forward_request_stream() 返回 Stream
   ↓
5. 逐块接收上游响应
   ↓
6. streaming::format_sse_chunk() 格式化为 SSE
   ↓
7. 流式返回给客户端
```

## 并发模型

- **异步运行时**: Tokio
- **HTTP 服务器**: Hyper (每个连接一个 task)
- **HTTP 客户端**: Reqwest (连接池)
- **状态共享**: Arc<Config> (不可变共享)

## 性能优化

1. **连接池**: Reqwest 默认连接池 (10 个空闲连接/主机)
2. **零拷贝**: 使用 Bytes 避免内存拷贝
3. **编译优化**: LTO + 单 codegen unit
4. **二进制优化**: strip + panic=abort
