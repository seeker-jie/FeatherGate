/**
 * FeatherGate JavaScript 客户端示例
 * 使用 fetch API
 */

const BASE_URL = 'http://localhost:8080';

// 基础聊天示例
async function basicChat() {
  console.log('=== 基础聊天示例 ===');

  const response = await fetch(`${BASE_URL}/v1/chat/completions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: 'gpt-4',
      messages: [{ role: 'user', content: 'Hello!' }]
    })
  });

  const data = await response.json();
  console.log(data.choices[0].message.content);
  console.log();
}

// 流式响应示例
async function streamingChat() {
  console.log('=== 流式响应示例 ===');

  const response = await fetch(`${BASE_URL}/v1/chat/completions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: 'gpt-4',
      messages: [{ role: 'user', content: 'Count to 5' }],
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

        try {
          const json = JSON.parse(data);
          const content = json.choices[0]?.delta?.content;
          if (content) process.stdout.write(content);
        } catch (e) {
          // 忽略解析错误
        }
      }
    }
  }
  console.log('\n');
}

// 运行示例
(async () => {
  await basicChat();
  await streamingChat();
})();
