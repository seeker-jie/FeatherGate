# FeatherGate 开发指南

## 开发环境设置

### 1. 安装依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/yourusername/feathergate
cd feathergate
```

### 2. 开发构建

```bash
# 开发模式构建（快速，包含调试信息）
cargo build

# 运行
./target/debug/feathergate --config test-config.yaml
```

## 测试

### 运行所有测试

```bash
cargo test --release
```

### 运行特定模块测试

```bash
# 配置模块
cargo test --release config::

# 提供商模块
cargo test --release providers::

# 服务器模块
cargo test --release server::
```

### 运行单个测试

```bash
cargo test --release test_forward_request_success
```

### 查看测试输出

```bash
cargo test --release -- --nocapture
```

## 代码质量

### Clippy 检查

```bash
cargo clippy --release -- -D warnings
```

### 代码格式化

```bash
# 检查格式
cargo fmt -- --check

# 自动格式化
cargo fmt
```

## 性能测试

### 运行基准测试

```bash
cargo bench
```

### 查看基准测试报告

```bash
open target/criterion/report/index.html
```

## 添加新的提供商

### 1. 创建提供商模块

在 `src/providers/` 下创建新文件，例如 `azure.rs`。

### 2. 实现核心函数

```rust
use crate::{config::ModelConfig, error::Result, types::*};

pub async fn forward_request(
    config: &ModelConfig,
    req: ChatRequest,
) -> Result<ChatResponse> {
    // 1. 转换请求格式
    let azure_req = convert_request(&req);

    // 2. 发送 HTTP 请求
    let response = send_request(config, azure_req).await?;

    // 3. 转换响应格式
    let chat_response = convert_response(response)?;

    Ok(chat_response)
}
```

### 3. 注册提供商

在 `src/providers/routing.rs` 中添加：

```rust
"azure" => azure::forward_request(config, req).await,
```

### 4. 添加测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_forward_request_success() {
        // 测试代码
    }
}
```

## 调试技巧

### 启用详细日志

```bash
RUST_LOG=debug cargo run -- --config test-config.yaml
```

### 使用 rust-gdb

```bash
rust-gdb target/debug/feathergate
```

## 项目结构

```
feathergate/
├── src/
│   ├── main.rs           # 入口点
│   ├── lib.rs            # 库入口
│   ├── config/           # 配置模块
│   ├── error.rs          # 错误处理
│   ├── types/            # 类型定义
│   ├── server/           # HTTP 服务器
│   ├── providers/        # 提供商实现
│   └── metrics/          # 监控指标
├── tests/                # 集成测试
├── benches/              # 性能测试
├── docs/                 # 文档
└── Cargo.toml            # 项目配置
```
