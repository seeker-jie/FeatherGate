use clap::Parser;
use feathergate::config::Config;
use feathergate::server;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "feathergate")]
#[command(about = "轻量级 LLM 代理服务", long_about = None)]
struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "feathergate.yaml")]
    config: String,

    /// 监听地址
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 解析命令行参数
    let args = Args::parse();

    // 加载配置
    let config = Config::from_file(&args.config)?;
    let config = Arc::new(config);

    // 解析监听地址
    let addr: SocketAddr = args.bind.parse()?;

    // 启动服务器
    server::start_server(config, addr).await?;

    Ok(())
}
