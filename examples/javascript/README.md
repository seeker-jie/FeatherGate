# JavaScript 客户端示例

使用 JavaScript 与 FeatherGate API 交互的示例。

## 文件说明

- `javascript_client.js` - Node.js 客户端示例

## 使用方法

```bash
# 安装依赖
npm install node-fetch

# 运行示例
node javascript_client.js
```

## 功能特性

- 支持 Node.js 环境
- 自动检测流式/非流式响应
- 完整的错误处理
- 类型安全的请求构建

## 示例代码结构

```javascript
// 发送聊天请求
async function chatCompletion(message, stream = false) {
  const response = await fetch('http://localhost:8080/v1/chat/completions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: 'gpt-4',
      messages: [{ role: 'user', content: message }],
      stream: stream
    })
  });

  if (stream) {
    // 处理流式响应
    const reader = response.body.getReader();
    // ... 流式处理逻辑
  } else {
    // 处理非流式响应
    const result = await response.json();
    return result;
  }
}
```