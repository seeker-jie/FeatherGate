# FeatherGate 部署指南

## 系统要求

- **操作系统**: Linux (Ubuntu 20.04+) 或 Windows 10+
- **内存**: 最低 50MB，推荐 100MB+
- **磁盘**: 10MB（二进制文件 + 配置）
- **网络**: 需要访问上游 LLM API

## 从源码构建

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 克隆仓库

```bash
git clone https://github.com/yourusername/feathergate
cd feathergate
```

### 3. 构建 Release 版本

```bash
cargo build --release
```

二进制文件位于 `target/release/feathergate`。

### 4. 验证构建

```bash
./target/release/feathergate --help
```

## 配置部署

### 1. 创建配置文件

```bash
cat > feathergate.yaml <<EOF
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: \${OPENAI_API_KEY}
EOF
```

### 2. 设置环境变量

```bash
export OPENAI_API_KEY="sk-xxx"
export ANTHROPIC_API_KEY="sk-ant-xxx"
export GEMINI_API_KEY="AIza-xxx"
```

### 3. 启动服务

```bash
./target/release/feathergate --config feathergate.yaml --bind 0.0.0.0:8080
```

## 生产部署

### 使用 systemd (Linux)

创建服务文件 `/etc/systemd/system/feathergate.service`:

```ini
[Unit]
Description=FeatherGate LLM Proxy
After=network.target

[Service]
Type=simple
User=feathergate
WorkingDirectory=/opt/feathergate
Environment="OPENAI_API_KEY=sk-xxx"
Environment="ANTHROPIC_API_KEY=sk-ant-xxx"
Environment="RUST_LOG=info"
ExecStart=/opt/feathergate/feathergate --config /opt/feathergate/feathergate.yaml
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

启动服务:

```bash
sudo systemctl daemon-reload
sudo systemctl enable feathergate
sudo systemctl start feathergate
sudo systemctl status feathergate
```

### 使用 Docker

创建 `Dockerfile`:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/feathergate /usr/local/bin/
COPY feathergate.yaml /etc/feathergate/
EXPOSE 8080
CMD ["feathergate", "--config", "/etc/feathergate/feathergate.yaml"]
```

构建和运行:

```bash
docker build -t feathergate .
docker run -d \
  -p 8080:8080 \
  -e OPENAI_API_KEY=sk-xxx \
  -e ANTHROPIC_API_KEY=sk-ant-xxx \
  --name feathergate \
  feathergate
```

## 监控和日志

### 日志配置

设置日志级别:

```bash
export RUST_LOG=info  # trace, debug, info, warn, error
./feathergate
```

### Prometheus 监控

FeatherGate 在 `/metrics` 端点暴露 Prometheus 指标。

Prometheus 配置示例:

```yaml
scrape_configs:
  - job_name: 'feathergate'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### 健康检查

```bash
curl http://localhost:8080/health
```

## 性能优化

### 1. 连接池配置

FeatherGate 默认使用连接池，每个主机最多保持 10 个空闲连接。

### 2. 资源限制

使用 systemd 限制资源:

```ini
[Service]
MemoryMax=100M
CPUQuota=50%
```

### 3. 反向代理

使用 Nginx 作为反向代理:

```nginx
upstream feathergate {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name api.example.com;

    location / {
        proxy_pass http://feathergate;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_buffering off;
    }
}
```

## 故障排查

### 服务无法启动

检查配置文件:
```bash
./feathergate --config feathergate.yaml
```

### 上游 API 错误

检查日志:
```bash
RUST_LOG=debug ./feathergate
```

### 性能问题

运行基准测试:
```bash
cargo bench
```
