#!/usr/bin/env python3
"""
FeatherGate Python 客户端示例
使用 requests 直接连接 FeatherGate
"""

import requests
import json
from typing import List, Dict, Any, Iterator


class FeatherGateClient:
    def __init__(self, base_url: str = "http://localhost:8080/v1"):
        self.base_url = base_url.rstrip("/")
        self.session = requests.Session()

    def chat(
        self,
        messages: List[Dict[str, str]],
        model: str = "gpt-4",
        stream: bool = False,
        **kwargs,
    ) -> Any:
        url = f"{self.base_url}/chat/completions"

        data = {"model": model, "messages": messages, "stream": stream, **kwargs}

        response = self.session.post(url, json=data)
        response.raise_for_status()

        if stream:
            return self._handle_stream(response)
        else:
            return response.json()

    def _handle_stream(self, response) -> Iterator[Dict[str, Any]]:
        for line in response.iter_lines():
            if line.startswith("data: "):
                data = line[6:]
                if data == "[DONE]":
                    break
                try:
                    yield json.loads(data)
                except json.JSONDecodeError:
                    continue


def example_basic():
    print("=== 基础聊天示例 ===")
    client = FeatherGateClient()

    messages = [{"role": "user", "content": "Hello, FeatherGate!"}]

    try:
        result = client.chat(messages)
        print("Response:", result["choices"][0]["message"]["content"])
    except Exception as e:
        print("Error:", e)


def example_streaming():
    print("=== 流式聊天示例 ===")
    client = FeatherGateClient()

    messages = [{"role": "user", "content": "写一首关于春天的诗"}]

    try:
        for chunk in client.chat(messages, stream=True):
            if "choices" in chunk and chunk["choices"]:
                delta = chunk["choices"][0].get("delta", {})
                content = delta.get("content", "")
                if content:
                    print(content, end="", flush=True)
        print()  # 换行
    except Exception as e:
        print("Error:", e)


def example_list_models():
    print("=== 列出模型示例 ===")
    client = FeatherGateClient()

    try:
        response = client.session.get(f"{client.base_url}/models")
        models = response.json()
        print("Available models:")
        for model in models.get("data", []):
            print(f"  - {model['id']}")
    except Exception as e:
        print("Error:", e)


def example_health_check():
    print("=== 健康检查示例 ===")
    client = FeatherGateClient()

    try:
        response = client.session.get(f"{client.base_url}/health")
        health = response.json()
        print("Health status:", health.get("status", "unknown"))
    except Exception as e:
        print("Error:", e)


if __name__ == "__main__":
    # 运行所有示例
    example_basic()
    print()
    example_streaming()
    print()
    example_list_models()
    print()
    example_health_check()
