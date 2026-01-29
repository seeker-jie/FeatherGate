# Shell/curl 示例

使用 curl 命令与 FeatherGate API 交互的示例。

## 文件说明

- `curl_examples.sh` - 包含各种 curl 示例脚本

## 使用方法

```bash
# 使脚本可执行
chmod +x curl_examples.sh

# 运行示例
./curl_examples.sh
```

## 支持的API端点

- `POST /v1/chat/completions` - 聊天完成（非流式）
- `POST /v1/chat/completions` - 聊天完成（流式，`stream: true`）
- `GET /v1/models` - 列出所有可用模型
- `GET /health` - 健康检查
- `GET /metrics` - Prometheus 格式的指标

## 示例请求

```bash
# 非流式请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# 流式请求
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": true
  }'
```