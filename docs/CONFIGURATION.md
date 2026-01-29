# FeatherGate 配置指南

## 配置文件格式

FeatherGate 使用 YAML 格式的配置文件，完全兼容 litellm 的配置格式。

## 基础配置

### 最小配置示例

```yaml
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: sk-xxx
```

### 完整配置示例

```yaml
model_list:
  # OpenAI 模型
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: ${OPENAI_API_KEY}
      api_base: https://api.openai.com/v1

  - model_name: gpt-4-turbo
    litellm_params:
      model: openai/gpt-4-turbo-preview
      api_key: ${OPENAI_API_KEY}

  # Anthropic Claude 模型
  - model_name: claude-opus
    litellm_params:
      model: anthropic/claude-opus-4-5
      api_key: ${ANTHROPIC_API_KEY}
      api_base: https://api.anthropic.com

  - model_name: claude-sonnet
    litellm_params:
      model: anthropic/claude-sonnet-4-5
      api_key: ${ANTHROPIC_API_KEY}

  # Google Gemini 模型
  - model_name: gemini-pro
    litellm_params:
      model: gemini/gemini-pro
      api_key: ${GEMINI_API_KEY}
      api_base: https://generativelanguage.googleapis.com
```

## 配置字段说明

### model_list

模型列表，每个模型包含以下字段：

#### model_name (必需)

客户端请求时使用的模型名称。

- 类型: `string`
- 示例: `"gpt-4"`, `"claude-opus"`, `"my-custom-model"`
- 说明: 可以自定义任意名称，客户端使用此名称发起请求

#### litellm_params (必需)

模型的详细参数配置。

##### model (必需)

提供商和模型 ID，格式为 `provider/model-id`。

- 类型: `string`
- 格式: `provider/model-id`
- 支持的 provider:
  - `openai` - OpenAI 模型
  - `anthropic` - Anthropic Claude 模型
  - `gemini` - Google Gemini 模型

示例:
```yaml
model: openai/gpt-4
model: anthropic/claude-opus-4-5
model: gemini/gemini-pro
```

##### api_key (必需)

上游 API 的密钥。

- 类型: `string`
- 支持环境变量: `${VAR_NAME}`
- 环境变量命名规则: 仅大写字母、数字和下划线
- 示例:
  ```yaml
    api_key: sk-xxx                    # 直接配置
    api_key: ${OPENAI_API_KEY}         # 环境变量
    api_key: ${API_KEY_123}           # 包含数字
    api_key: ${CUSTOM_VAR_NAME}         # 自定义变量名
  ```

## 配置验证规则

### 必需字段验证
- `model_list`: 不能为空数组
- 每个模型的 `model_name`: 不能为空字符串
- 每个模型的 `model`: 必须格式为 `provider/model-id`
- 每个模型的 `api_key`: 不能为空字符串

### model 格式验证
- 必须包含 `/` 分隔符
- provider 部分必须为: `openai`, `anthropic`, `gemini`
- model-id 部分不能为空

### 环境变量解析
- 使用正则表达式 `\$\{([A-Z0-9_]+)\}` 匹配
- 支持嵌套环境变量引用
- 未定义的环境变量会导致启动失败

### 默认 API Base URLs
如果 `api_base` 未指定，使用以下默认值：
- OpenAI: `https://api.openai.com/v1`
- Anthropic: `https://api.anthropic.com`
- Gemini: `https://generativelanguage.googleapis.com`

##### api_base (可选)

上游 API 的基础 URL。

- 类型: `string`
- 默认值:
  - OpenAI: `https://api.openai.com/v1`
  - Anthropic: `https://api.anthropic.com`
  - Gemini: `https://generativelanguage.googleapis.com`
- 用途: 自定义 API 端点（如使用代理或自托管服务）

## 环境变量

### 配置文件中的环境变量

使用 `${VAR_NAME}` 格式引用环境变量：

```yaml
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: ${OPENAI_API_KEY}
```

设置环境变量：

```bash
export OPENAI_API_KEY="sk-xxx"
export ANTHROPIC_API_KEY="sk-ant-xxx"
export GEMINI_API_KEY="AIza-xxx"
```

### 运行时环境变量

#### RUST_LOG

控制日志级别。

- 可选值: `trace`, `debug`, `info`, `warn`, `error`
- 默认值: `info`
- 示例:
  ```bash
  RUST_LOG=debug ./feathergate
  ```

## 命令行参数

### --config

指定配置文件路径。

```bash
./feathergate --config /path/to/config.yaml
```

默认值: `feathergate.yaml`

### --bind

指定服务器监听地址。

```bash
./feathergate --bind 0.0.0.0:8080
```

默认值: `0.0.0.0:8080`

## 配置示例

### 多个 OpenAI 模型

```yaml
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: ${OPENAI_API_KEY}

  - model_name: gpt-3.5
    litellm_params:
      model: openai/gpt-3.5-turbo
      api_key: ${OPENAI_API_KEY}
```

### 使用自定义 API 端点

```yaml
model_list:
  - model_name: my-gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: ${CUSTOM_API_KEY}
      api_base: https://my-proxy.example.com/v1
```

### 混合多个提供商

```yaml
model_list:
  - model_name: fast-model
    litellm_params:
      model: openai/gpt-3.5-turbo
      api_key: ${OPENAI_API_KEY}

  - model_name: smart-model
    litellm_params:
      model: anthropic/claude-opus-4-5
      api_key: ${ANTHROPIC_API_KEY}

  - model_name: free-model
    litellm_params:
      model: gemini/gemini-pro
      api_key: ${GEMINI_API_KEY}
```

## 配置验证

启动时，FeatherGate 会验证配置：

1. **必需字段检查**: 确保所有必需字段存在
2. **格式验证**: 验证 `model` 字段格式为 `provider/model-id`
3. **环境变量检查**: 验证所有引用的环境变量已设置
4. **非空检查**: 确保 `model_list` 不为空

如果配置无效，服务器将拒绝启动并显示错误信息。

## 最佳实践

1. **使用环境变量存储密钥**: 避免在配置文件中硬编码 API 密钥
2. **使用有意义的 model_name**: 便于客户端识别和使用
3. **保持配置文件简洁**: 只配置实际使用的模型
4. **定期更新模型 ID**: 跟随上游提供商的模型更新
