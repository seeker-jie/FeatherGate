# Changelog

All notable changes to the FeatherGate project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-01-28

### Added
- 完整的 OpenAI、Anthropic Claude、Google Gemini 支持
- OpenAI 兼容的 REST API
- 流式响应支持（SSE）
- 配置文件支持（YAML，兼容 litellm 格式）
- Prometheus 指标端点
- 健康检查端点
- 模型列表端点
- 完整的错误处理
- 单元测试和集成测试（54个测试，100%通过）
- 性能基准测试

### Features
- 轻量级二进制（4.9MB）
- 低内存占用（~7MB RSS）
- 快速启动（8ms）
- 低代理开销（<5ms）
- HTTP 连接池
- 优雅关闭（SIGTERM/Ctrl+C）
- 环境变量替换支持

### API Endpoints
- `POST /v1/chat/completions` - 聊天完成（支持流式）
- `GET /v1/models` - 列出模型
- `GET /health` - 健康检查
- `GET /metrics` - Prometheus 指标

### Supported Providers
- **OpenAI**: GPT-3.5-turbo, GPT-4 系列（直接 passthrough）
- **Anthropic**: Claude-Opus, Claude-Sonnet, Claude-Haiku（协议转换）
- **Google Gemini**: Gemini Pro, Gemini 3 Flash（协议转换）

### Configuration
- 完全兼容 litellm 配置格式
- 支持环境变量替换 `${VAR_NAME}`
- 可配置的监听地址和超时
- JSON 和 Pretty 日志格式支持

### Performance
- 二进制大小: 4.9MB
- 内存占用: ~7MB RSS
- 启动时间: 8ms
- 代理开销: <5ms
- 测试覆盖率: 54个测试，100%通过

### Documentation
- 完整的 README.md
- API 文档
- 架构文档
- 配置指南
- 部署指南
- 开发指南
- 客户端示例（Shell、JavaScript、Python）