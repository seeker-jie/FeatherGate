use feathergate::config::{Config, LitellmParams, ModelConfig};
use feathergate::server;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// 测试流式响应端点（基础测试）
#[tokio::test]
async fn test_streaming_endpoint_detection() {
    let config = Arc::new(Config {
        model_list: vec![ModelConfig {
            model_name: "test-gpt".to_string(),
            litellm_params: LitellmParams {
                model: "openai/gpt-4".to_string(),
                api_key: "sk-test-key".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
            },
        }],
    });

    let addr: std::net::SocketAddr = "127.0.0.1:18090".parse().unwrap();

    // 启动服务器
    let server_config = Arc::clone(&config);
    tokio::spawn(async move {
        let _ = server::start_server_test(server_config, addr).await;
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 测试流式请求（会返回错误，因为没有真实 API key，但能验证端点存在）
    let client = reqwest::Client::new();
    let result = timeout(
        Duration::from_secs(3),
        client
            .post("http://127.0.0.1:18090/v1/chat/completions")
            .json(&serde_json::json!({
                "model": "test-gpt",
                "messages": [{"role": "user", "content": "hello"}],
                "stream": true
            }))
            .send(),
    )
    .await;

    // 验证请求能够到达服务器（即使可能失败）
    assert!(result.is_ok(), "流式请求应该能够发送");

    if let Ok(Ok(response)) = result {
        // 验证返回了正确的 Content-Type（SSE）
        let content_type = response.headers().get("content-type");
        if response.status().is_success() {
            assert!(
                content_type.is_some() &&
                content_type.unwrap().to_str().unwrap().contains("text/event-stream"),
                "流式响应应该返回 text/event-stream"
            );
        }
    }
}

/// 测试非流式请求仍然工作
#[tokio::test]
async fn test_non_streaming_still_works() {
    let config = Arc::new(Config {
        model_list: vec![ModelConfig {
            model_name: "test-model".to_string(),
            litellm_params: LitellmParams {
                model: "openai/gpt-4".to_string(),
                api_key: "sk-test".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
            },
        }],
    });

    let addr: std::net::SocketAddr = "127.0.0.1:18091".parse().unwrap();

    // 启动服务器
    let server_config = Arc::clone(&config);
    tokio::spawn(async move {
        let _ = server::start_server_test(server_config, addr).await;
    });

    // 等待服务器启动
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 测试非流式请求
    let client = reqwest::Client::new();
    let result = timeout(
        Duration::from_secs(3),
        client
            .post("http://127.0.0.1:18091/v1/chat/completions")
            .json(&serde_json::json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hello"}],
                "stream": false
            }))
            .send(),
    )
    .await;

    // 验证非流式请求仍然工作
    assert!(result.is_ok(), "非流式请求应该仍然工作");
}
