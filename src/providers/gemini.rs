use crate::config::{parse_model_string, ModelConfig};
use crate::error::FeatherGateError;
use crate::types::{ChatRequest, ChatResponse, Choice, Message, Usage};
use crate::Result;
use futures_util::Stream;
use hyper::body::Bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

/// 获取全局 HTTP 客户端
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

/// Gemini API 请求格式
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

/// Gemini API 响应格式
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: GeminiContentResponse,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}

/// 转换 OpenAI 请求为 Gemini 格式
fn convert_request(req: &ChatRequest) -> GeminiRequest {
    let mut contents = Vec::new();
    let mut system_content = None;

    // 提取并合并 system message
    for msg in &req.messages {
        if msg.role == "system" {
            system_content = Some(msg.content.clone());
        } else {
            let role = if msg.role == "assistant" {
                "model"
            } else {
                &msg.role
            };

            let mut text = msg.content.clone();

            // 如果是第一个 user message，合并 system message
            if role == "user" && system_content.is_some() && contents.is_empty() {
                text = format!("{}\n\n{}", system_content.take().unwrap(), text);
            }

            contents.push(GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart { text }],
            });
        }
    }

    let generation_config = if req.temperature.is_some()
        || req.max_tokens.is_some()
        || req.top_p.is_some()
    {
        Some(GenerationConfig {
            temperature: req.temperature,
            max_output_tokens: req.max_tokens,
            top_p: req.top_p,
        })
    } else {
        None
    };

    GeminiRequest {
        contents,
        generation_config,
    }
}

/// 转换 Gemini 响应为 OpenAI 格式
fn convert_response(resp: GeminiResponse, model: &str) -> Result<ChatResponse> {
    let candidate = resp
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| FeatherGateError::internal("Gemini 响应中没有 candidates"))?;

    // 提取文本内容
    let content = candidate
        .content
        .parts
        .into_iter()
        .map(|part| part.text)
        .collect::<Vec<_>>()
        .join("");

    // 转换 finish_reason
    let finish_reason = candidate.finish_reason.map(|reason| match reason.as_str() {
        "STOP" => "stop".to_string(),
        "MAX_TOKENS" => "length".to_string(),
        _ => reason,
    });

    let usage = resp.usage_metadata.map(|meta| Usage {
        prompt_tokens: meta.prompt_token_count,
        completion_tokens: meta.candidates_token_count,
        total_tokens: meta.total_token_count,
    });

    Ok(ChatResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: model.to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message::assistant(content),
            finish_reason,
        }],
        usage,
    })
}

/// 转发请求到 Gemini
pub async fn forward_request(
    config: &ModelConfig,
    req: &ChatRequest,
) -> Result<ChatResponse> {
    let client = get_http_client();

    // 解析模型 ID（使用统一的解析函数）
    let (_, model_id) = parse_model_string(&config.litellm_params.model)?;

    // 转换请求
    let gemini_req = convert_request(req);

    // 构建 URL（不在 URL 中暴露 API 密钥）
    let api_base = if config.litellm_params.api_base.is_empty() {
        "https://generativelanguage.googleapis.com"
    } else {
        &config.litellm_params.api_base
    };
    let url = format!(
        "{}/v1beta/models/{}:generateContent",
        api_base.trim_end_matches('/'),
        model_id
    );

    // 发送请求（通过 HTTP 头传递 API 密钥）
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", &config.litellm_params.api_key)
        .json(&gemini_req)
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
            format!("Gemini API 错误: {}", error_body),
        ));
    }

    // 解析响应
    let gemini_resp: GeminiResponse = response.json().await?;
    convert_response(gemini_resp, &model_id)
}

/// 转发流式请求到 Gemini
pub async fn forward_request_stream(
    config: &ModelConfig,
    req: &ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>> {
    let client = get_http_client();

    // 解析模型 ID
    let (_, model_id) = parse_model_string(&config.litellm_params.model)?;

    // 转换请求
    let gemini_req = convert_request(req);

    // 构建流式 URL
    let api_base = if config.litellm_params.api_base.is_empty() {
        "https://generativelanguage.googleapis.com"
    } else {
        &config.litellm_params.api_base
    };
    let url = format!(
        "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
        api_base.trim_end_matches('/'),
        model_id
    );

    // 发送请求
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", &config.litellm_params.api_key)
        .json(&gemini_req)
        .send()
        .await?;

    // 检查状态码
    let status = response.status();
    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(4096)
            .collect::<String>();
        return Err(FeatherGateError::upstream(
            status.as_u16(),
            format!("Gemini API 错误: {}", error_body),
        ));
    }

    // 创建 SSE 转换流
    let model_id_owned = model_id.clone();
    let stream = create_gemini_stream(response, model_id_owned);

    Ok(Box::pin(stream))
}

/// 创建 Gemini SSE 转换流
fn create_gemini_stream(
    response: reqwest::Response,
    model_id: String,
) -> impl Stream<Item = Result<Bytes>> + Send + Sync {
    use futures_util::StreamExt;

    let mut buffer = String::new();
    let chunk_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());

    response.bytes_stream().filter_map(move |result| {
        let output = match result {
            Ok(bytes) => {
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                process_gemini_buffer(&mut buffer, &chunk_id, &model_id)
            }
            Err(e) => Some(Err(FeatherGateError::HttpError(e))),
        };
        std::future::ready(output)
    })
}

/// 处理 Gemini SSE 缓冲区
fn process_gemini_buffer(
    buffer: &mut String,
    chunk_id: &str,
    model_id: &str,
) -> Option<Result<Bytes>> {
    // Gemini SSE 格式: data: {...}\n\n
    while let Some(pos) = buffer.find("\n\n") {
        let line = buffer[..pos].to_string();
        *buffer = buffer[pos + 2..].to_string();

        if let Some(data) = line.strip_prefix("data: ") {
            if let Some(chunk) = parse_gemini_chunk(data, chunk_id, model_id) {
                return Some(Ok(chunk));
            }
        }
    }
    None
}

/// 解析 Gemini 响应块并转换为 OpenAI 格式
fn parse_gemini_chunk(data: &str, chunk_id: &str, model_id: &str) -> Option<Bytes> {
    // 解析 Gemini 响应
    let resp: GeminiResponse = serde_json::from_str(data).ok()?;

    // 提取文本内容
    let candidate = resp.candidates.first()?;
    let text = candidate.content.parts.first()?.text.clone();

    // 检查是否结束
    let finish_reason = candidate.finish_reason.as_ref().map(|r| match r.as_str() {
        "STOP" => "stop",
        "MAX_TOKENS" => "length",
        _ => "stop",
    });

    // 创建 OpenAI 格式的 chunk
    let chunk = create_gemini_openai_chunk(chunk_id, model_id, &text, finish_reason);
    Some(Bytes::from(chunk))
}

/// 创建 OpenAI 格式的 SSE chunk
fn create_gemini_openai_chunk(
    id: &str,
    model: &str,
    content: &str,
    finish_reason: Option<&str>,
) -> String {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let escaped = escape_json_gemini(content);
    let delta = format!(r#"{{"content":"{}"}}"#, escaped);

    let finish = match finish_reason {
        Some(r) => format!(r#""{}""#, r),
        None => "null".to_string(),
    };

    format!(
        r#"data: {{"id":"{}","object":"chat.completion.chunk","created":{},"model":"{}","choices":[{{"index":0,"delta":{},"finish_reason":{}}}]}}

"#,
        id, created, model, delta, finish
    )
}

/// 转义 JSON 字符串
fn escape_json_gemini(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LitellmParams;
    use mockito::{Server, ServerGuard};

    async fn setup_mock_server() -> ServerGuard {
        Server::new_async().await
    }

    fn create_test_config(api_base: &str) -> ModelConfig {
        ModelConfig {
            model_name: "gemini".to_string(),
            litellm_params: LitellmParams {
                model: "gemini/gemini-pro".to_string(),
                api_key: "test-api-key".to_string(),
                api_base: api_base.to_string(),
            },
        }
    }

    #[test]
    fn test_convert_request_basic() {
        let req = ChatRequest {
            model: "gemini".to_string(),
            messages: vec![Message::user("Hello")],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
            top_p: None,
        };

        let gemini_req = convert_request(&req);

        assert_eq!(gemini_req.contents.len(), 1);
        assert_eq!(gemini_req.contents[0].role, "user");
        assert_eq!(gemini_req.contents[0].parts[0].text, "Hello");
        assert!(gemini_req.generation_config.is_some());
        assert_eq!(
            gemini_req.generation_config.as_ref().unwrap().temperature,
            Some(0.7)
        );
    }

    #[test]
    fn test_convert_request_with_system() {
        let req = ChatRequest {
            model: "gemini".to_string(),
            messages: vec![
                Message::system("You are helpful"),
                Message::user("Hello"),
            ],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };

        let gemini_req = convert_request(&req);

        assert_eq!(gemini_req.contents.len(), 1);
        assert_eq!(gemini_req.contents[0].role, "user");
        // system message 应该被合并到第一个 user message
        assert!(gemini_req.contents[0].parts[0].text.contains("You are helpful"));
        assert!(gemini_req.contents[0].parts[0].text.contains("Hello"));
    }

    #[test]
    fn test_convert_request_role_mapping() {
        let req = ChatRequest {
            model: "gemini".to_string(),
            messages: vec![Message::user("Hi"), Message::assistant("Hello")],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };

        let gemini_req = convert_request(&req);

        assert_eq!(gemini_req.contents.len(), 2);
        assert_eq!(gemini_req.contents[0].role, "user");
        assert_eq!(gemini_req.contents[1].role, "model"); // assistant -> model
    }

    #[test]
    fn test_convert_response() {
        let gemini_resp = GeminiResponse {
            candidates: vec![Candidate {
                content: GeminiContentResponse {
                    parts: vec![GeminiPartResponse {
                        text: "Hello from Gemini!".to_string(),
                    }],
                },
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: Some(UsageMetadata {
                prompt_token_count: 10,
                candidates_token_count: 20,
                total_token_count: 30,
            }),
        };

        let openai_resp = convert_response(gemini_resp, "gemini-pro").unwrap();

        assert_eq!(openai_resp.object, "chat.completion");
        assert_eq!(openai_resp.model, "gemini-pro");
        assert_eq!(openai_resp.choices.len(), 1);
        assert_eq!(openai_resp.choices[0].message.content, "Hello from Gemini!");
        assert_eq!(openai_resp.choices[0].finish_reason, Some("stop".to_string()));
        assert_eq!(openai_resp.usage.as_ref().unwrap().total_tokens, 30);
    }

    #[tokio::test]
    async fn test_forward_request_success() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/v1beta/models/gemini-pro:generateContent")
            .match_header("x-goog-api-key", "test-api-key")
            .with_status(200)
            .with_body(
                r#"{
                "candidates": [{
                    "content": {
                        "parts": [{
                            "text": "Hello from Gemini!"
                        }]
                    },
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 5,
                    "candidatesTokenCount": 10,
                    "totalTokenCount": 15
                }
            }"#,
            )
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = ChatRequest {
            model: "gemini".to_string(),
            messages: vec![Message::user("Hello")],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
            top_p: None,
        };

        let result = forward_request(&config, &req).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.choices[0].message.content, "Hello from Gemini!");
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 15);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_forward_request_api_error() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/v1beta/models/gemini-pro:generateContent")
            .match_header("x-goog-api-key", "test-api-key")
            .with_status(400)
            .with_body(r#"{"error": {"message": "Invalid request"}}"#)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = ChatRequest {
            model: "gemini".to_string(),
            messages: vec![Message::user("Hello")],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };

        let result = forward_request(&config, &req).await;
        assert!(result.is_err());

        mock.assert_async().await;
    }
}
