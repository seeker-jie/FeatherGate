pub mod handlers;
pub mod streaming;

use crate::config::Config;
use crate::Result;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};

/// 启动 HTTP 服务器（带优雅关闭）
pub async fn start_server(config: Arc<Config>, addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("FeatherGate 服务器运行在 http://{}", addr);

    // 设置优雅关闭信号处理
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(());
    
    // 监听 Ctrl+C 信号
    #[cfg(unix)]
    {
        let sigterm = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("设置 SIGTERM 信号处理失败")
                .recv()
                .await;
        };
        
        let sigint = async {
            signal::ctrl_c().await.expect("设置 Ctrl+C 信号处理失败");
        };
        
        tokio::select! {
            _ = sigterm => {
                warn!("收到 SIGTERM 信号，开始优雅关闭...");
            }
            _ = sigint => {
                warn!("收到 Ctrl+C 信号，开始优雅关闭...");
            }
        }
        
        // 发送关闭信号
        let _ = shutdown_tx.send(());
    }
    
    #[cfg(not(unix))]
    {
        signal::ctrl_c().await.expect("设置 Ctrl+C 信号处理失败");
        warn!("收到 Ctrl+C 信号，开始优雅关闭...");
        let _ = shutdown_tx.send(());
    }

    // 启动服务器循环，等待关闭信号
    let server_handle = tokio::spawn({
        let mut shutdown_rx = shutdown_rx.clone();
        let listener = listener;
        let config = config.clone();
        
        async move {
            loop {
                tokio::select! {
                    // 等待新连接
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let io = TokioIo::new(stream);
                                let config = Arc::clone(&config);
                                
                                tokio::spawn(async move {
                                    let service = service_fn(move |req| {
                                        let config = Arc::clone(&config);
                                        handlers::handle_request(req, config)
                                    });
                                    
                                    if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                                        error!("服务连接错误: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("接受连接失败: {}", e);
                                break;
                            }
                        }
                    }
                    // 等待关闭信号
                    _ = shutdown_rx.changed() => {
                        info!("收到关闭信号，停止接受新连接");
                        break;
                    }
                }
            }
        }
    });

    // 等待关闭信号
    if let Err(e) = shutdown_rx.changed().await {
        error!("等待关闭信号时出错: {}", e);
        return Ok(());
    }
    
    // 等待服务器处理完现有连接
    info!("等待现有连接处理完成...");
    if let Err(e) = server_handle.await {
        error!("等待服务器关闭时出错: {}", e);
    }
    
    info!("服务器已优雅关闭");
    Ok(())
}

/// 启动 HTTP 服务器（仅用于测试，不监听关闭信号）
pub async fn start_server_test(config: Arc<Config>, addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("FeatherGate 测试服务器运行在 http://{}", addr);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("接受连接失败: {}", e);
                continue;
            }
        };

        let io = TokioIo::new(stream);
        let config = Arc::clone(&config);

        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let config = Arc::clone(&config);
                handlers::handle_request(req, config)
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                error!("服务连接错误: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LitellmParams, ModelConfig};
    use std::time::Duration;
    use tokio::time::timeout;

    fn create_test_config() -> Config {
        Config {
            model_list: vec![ModelConfig {
                model_name: "test-model".to_string(),
                litellm_params: LitellmParams {
                    model: "openai/gpt-4".to_string(),
                    api_key: "sk-test".to_string(),
                    api_base: "https://api.openai.com".to_string(),
                },
            }],
        }
    }

    #[tokio::test]
    async fn test_server_starts() {
        let config = Arc::new(create_test_config());
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        // 启动服务器，但立即超时（仅测试启动逻辑）
        let server_task = tokio::spawn(async move {
            let _ = start_server(config, addr).await;
        });

        // 等待短暂时间后取消
        tokio::time::sleep(Duration::from_millis(100)).await;
        server_task.abort();
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let config = Arc::new(create_test_config());
        let addr: SocketAddr = "127.0.0.1:18080".parse().unwrap();

        // 启动服务器
        let server_config = Arc::clone(&config);
        tokio::spawn(async move {
            let _ = start_server(server_config, addr).await;
        });

        // 等待服务器启动
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 测试健康检查端点
        let client = reqwest::Client::new();
        let result = timeout(
            Duration::from_secs(2),
            client.get("http://127.0.0.1:18080/health").send(),
        )
        .await;

        if let Ok(Ok(response)) = result {
            assert_eq!(response.status(), 200);
            let body: serde_json::Value = response.json().await.unwrap();
            assert_eq!(body["status"], "ok");
        }
    }
}
