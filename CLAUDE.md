# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FeatherGate is a lightweight, high-performance LLM proxy service written in Rust, designed as a drop-in replacement for litellm. It accepts OpenAI-compatible API requests and routes them to multiple LLM providers (OpenAI, Google Gemini, Anthropic Claude) with automatic protocol conversion.

## Build & Development Commands

```bash
# Build
cargo build --release

# Run
./target/release/feathergate --config feathergate.yaml

# Run all tests
cargo test --release

# Run a single test
cargo test --release test_name

# Run tests in a specific module
cargo test --release module_name::

# Performance benchmarks
cargo bench

# Windows build
cargo build --release
.\target\release\feathergate.exe --config feathergate.yaml
```

## Architecture

### Core Flow

Client (OpenAI format) → FeatherGate HTTP Server → Router → Provider Client → LLM API

All responses are converted back to OpenAI-compatible format regardless of the upstream provider.

### Key Components

- **HTTP Server Layer** (`src/server/`): Hyper + Tokio async server with graceful shutdown
- **Router** (`src/providers/routing.rs`): Maps `model_name` from request to `ModelConfig`, parses `provider/model-id` format to determine which provider client to use
- **Provider Clients** (`src/providers/`): Each provider (openai, anthropic, gemini) has its own module with `forward_request` function that handles protocol conversion. OpenAI is passthrough; Gemini and Claude require format translation
- **Configuration** (`src/config/`): YAML-based config compatible with litellm format (`model_list` with `litellm_params`)
- **Streaming**: SSE (Server-Sent Events) support — provider-specific stream chunks are converted to OpenAI SSE format on the fly

### Provider Pattern

Each provider module (openai, anthropic, gemini) exports:
- `forward_request(config: &ModelConfig, req: ChatRequest) -> Result<ChatResponse>`

This function:
1. Converts OpenAI `ChatRequest` → provider-native format
2. Sends HTTP request using config's `api_key` and `api_base`
3. Converts provider response → OpenAI `ChatResponse`

Router state (model configs) is wrapped in `Arc` for thread-safe access across async tasks.

### API Endpoints

- `POST /v1/chat/completions` — Chat completion (streaming via `"stream": true`)
- `GET /v1/models` — List supported models
- `GET /health` — Health check
- `GET /metrics` — Prometheus metrics

### Configuration

Main config file: `feathergate.yaml`. Format is fully compatible with litellm:
```yaml
model_list:
  - model_name: claude-opus
    litellm_params:
      model: anthropic/claude-opus-4-5
      api_key: sk-ant-xxx
      api_base: https://api.anthropic.com/
```

Router extracts provider from `model` field format (`provider/model-id`) and uses corresponding `api_key` and `api_base`.

### Performance Targets

- Startup: <100ms, Memory: <15MB, QPS: >5000, Proxy overhead: <5ms, Binary: <5MB

### Key Dependencies

Tokio (async runtime), Hyper (HTTP server), Reqwest (HTTP client), Serde (serialization), Tracing (structured logging), thiserror/anyhow (error handling)

### Compilation Profile

Release builds use LTO, single codegen unit, panic=abort, and binary stripping for minimal size.
