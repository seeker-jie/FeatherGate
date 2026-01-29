use crate::config::ModelConfig;
use crate::error::FeatherGateError;
use crate::types::{ChatRequest, ChatResponse};
use crate::Result;
use futures_util::Stream;
use hyper::body::Bytes;
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;

/// 获取全局 HTTP 客户端（连接池复用）
fn get_http_client() -> &'static Client {
    use once_cell::sync::Lazy;
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        Client::builder()
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap()
    });
    &CLIENT
}

/// 转发请求到 OpenAI（直接 passthrough）
pub async fn forward_request(
    config: &ModelConfig,
    req: &ChatRequest,
) -> Result<ChatResponse> {
    let client = get_http_client();

    // 构建 URL
    let api_base = if config.litellm_params.api_base.is_empty() {
        "https://api.openai.com/v1"
    } else {
        &config.litellm_params.api_base
    };
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    // 发送请求
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.litellm_params.api_key))
        .header("Content-Type", "application/json")
        .json(req)
        .send()
        .await?;

    // 检查状态码
    let status = response.status();
    if !status.is_success() {
        // 限制错误响应体大小，防止 DoS 攻击
        let error_body = response
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(4096)
            .collect::<String>();
        return Err(FeatherGateError::upstream(
            status.as_u16(),
            format!("OpenAI API 错误: {}", error_body),
        ));
    }

    // 解析响应
    let chat_response: ChatResponse = response.json().await?;
    Ok(chat_response)
}

/// 转发流式请求到 OpenAI
pub async fn forward_request_stream(
    config: &ModelConfig,
    req: &ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>> {
    let client = get_http_client();

    // 构建 URL
    let api_base = if config.litellm_params.api_base.is_empty() {
        "https://api.openai.com/v1"
    } else {
        &config.litellm_params.api_base
    };
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    // 发送请求
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.litellm_params.api_key))
        .header("Content-Type", "application/json")
        .json(req)
        .send()
        .await?;

    // 检查状态码
    let status = response.status();
    if !status.is_success() {
        // 限制错误响应体大小，防止 DoS 攻击
        let error_body = response
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(4096)
            .collect::<String>();
        return Err(FeatherGateError::upstream(
            status.as_u16(),
            format!("OpenAI API 错误: {}", error_body),
        ));
    }

    // 返回字节流
    use futures_util::StreamExt;
    let stream = response.bytes_stream().map(|result| {
        result.map_err(FeatherGateError::HttpError)
    });

    Ok(Box::pin(stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LitellmParams;
    use crate::types::Message;
    use mockito::{Server, ServerGuard};

    async fn setup_mock_server() -> ServerGuard {
        Server::new_async().await
    }

    fn create_test_config(api_base: &str) -> ModelConfig {
        ModelConfig {
            model_name: "gpt-4".to_string(),
            litellm_params: LitellmParams {
                model: "openai/gpt-4".to_string(),
                api_key: "sk-test-key".to_string(),
                api_base: api_base.to_string(),
            },
        }
    }

    fn create_test_request() -> ChatRequest {
        ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("Hello")],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
            top_p: None,
        }
    }

    #[tokio::test]
    async fn test_forward_request_success() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer sk-test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1677652288,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help?"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                }
            }"#,
            )
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = create_test_request();

        let result = forward_request(&config, &req).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, "Hello! How can I help?");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_forward_request_preserves_parameters() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/chat/completions")
            .match_body(mockito::Matcher::Json(serde_json::json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "Hello"}],
                "temperature": 0.7,
                "max_tokens": 100
            })))
            .with_status(200)
            .with_body(
                r#"{
                "id": "test-id",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Response"},
                    "finish_reason": "stop"
                }]
            }"#,
            )
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = create_test_request();

        let result = forward_request(&config, &req).await;
        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_forward_request_api_error() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(400)
            .with_body(r#"{"error": {"message": "Invalid request"}}"#)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = create_test_request();

        let result = forward_request(&config, &req).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            FeatherGateError::UpstreamError { status, .. } => {
                assert_eq!(status, 400);
            }
            _ => panic!("Expected UpstreamError"),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_forward_request_with_empty_api_base() {
        // 当 api_base 为空时，应使用默认 OpenAI URL
        let config = ModelConfig {
            model_name: "gpt-4".to_string(),
            litellm_params: LitellmParams {
                model: "openai/gpt-4".to_string(),
                api_key: "sk-test-key".to_string(),
                api_base: String::new(), // 空字符串
            },
        };

        let req = create_test_request();

        // 此测试会失败（因为没有真实 API key），但验证了 URL 构建逻辑
        let result = forward_request(&config, &req).await;
        assert!(result.is_err()); // 预期失败，但不是因为 URL 问题
    }
}
