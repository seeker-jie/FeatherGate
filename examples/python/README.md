# Python 客户端示例

使用 Python 与 FeatherGate API 交互的示例。

## 文件说明

- `python_client.py` - Python 客户端示例

## 使用方法

```bash
# 安装依赖
pip install requests

# 运行示例
python python_client.py
```

## 功能特性

- 支持同步和异步调用
- 自动处理流式响应
- 完整的异常处理
- 类型提示支持

## 示例代码结构

```python
import requests
import json

class FeatherGateClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url.rstrip('/')
    
    def chat_completion(self, messages, stream=False, **kwargs):
        """发送聊天完成请求"""
        url = f"{self.base_url}/v1/chat/completions"
        
        data = {
            "model": "gpt-4",
            "messages": messages,
            "stream": stream,
            **kwargs
        }
        
        response = requests.post(url, json=data)
        
        if stream:
            return self._handle_stream(response)
        else:
            return response.json()
    
    def _handle_stream(self, response):
        """处理流式响应"""
        for line in response.iter_lines():
            if line.startswith('data: '):
                data = line[6:]  # 移除 'data: ' 前缀
                if data == '[DONE]':
                    break
                yield json.loads(data)

# 使用示例
client = FeatherGateClient()
messages = [{"role": "user", "content": "Hello!"}]

# 非流式
result = client.chat_completion(messages)
print(result["choices"][0]["message"]["content"])

# 流式
for chunk in client.chat_completion(messages, stream=True):
    if "choices" in chunk and chunk["choices"]:
        content = chunk["choices"][0]["delta"].get("content", "")
        if content:
            print(content, end="", flush=True)
```

## 依赖

- `requests` - HTTP 客户端
- `asyncio` - 异步支持（可选）
- `typing` - 类型提示