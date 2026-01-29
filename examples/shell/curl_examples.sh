#!/bin/bash
# FeatherGate cURL 示例

BASE_URL="http://localhost:8080"

echo "=== 1. 健康检查 ==="
curl -s "${BASE_URL}/health" | jq .
echo -e "\n"

echo "=== 2. 列出模型 ==="
curl -s "${BASE_URL}/v1/models" | jq .
echo -e "\n"

echo "=== 3. 基础聊天 ==="
curl -s -X POST "${BASE_URL}/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }' | jq .
echo -e "\n"

echo "=== 4. 流式响应 ==="
curl -N -X POST "${BASE_URL}/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Count to 3"}],
    "stream": true
  }'
echo -e "\n"

echo "=== 5. Prometheus 指标 ==="
curl -s "${BASE_URL}/metrics"
