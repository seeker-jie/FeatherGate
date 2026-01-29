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

/// Anthropic API 请求格式
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic API 响应格式
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    response_type: String,
    #[allow(dead_code)]
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic SSE 事件类型
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        #[allow(dead_code)]
        index: u32,
        #[allow(dead_code)]
        content_block: ContentBlockData,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        #[allow(dead_code)]
        index: u32,
        delta: DeltaData,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        #[allow(dead_code)]
        index: u32,
    },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaData },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error {
        #[allow(dead_code)]
        error: ErrorData,
    },
}

#[derive(Debug, Deserialize)]
struct MessageStartData {
    id: String,
    #[allow(dead_code)]
    model: String,
}

#[derive(Debug, Deserialize)]
struct ContentBlockData {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    block_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum DeltaData {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        #[allow(dead_code)]
        partial_json: String,
    },
}

#[derive(Debug, Deserialize)]
struct MessageDeltaData {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorData {
    #[allow(dead_code)]
    message: String,
}

/// 转换 OpenAI 请求为 Anthropic 格式
fn convert_request(req: &ChatRequest, model_id: &str) -> AnthropicRequest {
    // 提取 system message
    let mut system_message = None;
    let mut messages = Vec::new();

    for msg in &req.messages {
        if msg.role == "system" {
            system_message = Some(msg.content.clone());
        } else {
            messages.push(AnthropicMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }
    }

    AnthropicRequest {
        model: model_id.to_string(),
        messages,
        system: system_message,
        max_tokens: req.max_tokens.unwrap_or(1024),
        temperature: req.temperature,
        stream: None,
    }
}

/// 转换为流式请求
fn convert_request_stream(req: &ChatRequest, model_id: &str) -> AnthropicRequest {
    let mut base = convert_request(req, model_id);
    base.stream = Some(true);
    base
}

/// 转换 Anthropic 响应为 OpenAI 格式
fn convert_response(resp: AnthropicResponse) -> ChatResponse {
    // 提取文本内容
    let content = resp
        .content
        .into_iter()
        .filter(|block| block.block_type == "text")
        .map(|block| block.text)
        .collect::<Vec<_>>()
        .join("");

    // 转换 finish_reason
    let finish_reason = resp.stop_reason.map(|reason| match reason.as_str() {
        "end_turn" => "stop".to_string(),
        "max_tokens" => "length".to_string(),
        _ => reason,
    });

    ChatResponse {
        id: resp.id,
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: resp.model,
        choices: vec![Choice {
            index: 0,
            message: Message::assistant(content),
            finish_reason,
        }],
        usage: Some(Usage {
            prompt_tokens: resp.usage.input_tokens,
            completion_tokens: resp.usage.output_tokens,
            total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
        }),
    }
}

/// 转发请求到 Anthropic
pub async fn forward_request(
    config: &ModelConfig,
    req: &ChatRequest,
) -> Result<ChatResponse> {
    let client = get_http_client();

    // 解析模型 ID（使用统一的解析函数）
    let (_, model_id) = parse_model_string(&config.litellm_params.model)?;

    // 转换请求
    let anthropic_req = convert_request(req, &model_id);

    // 构建 URL
    let api_base = if config.litellm_params.api_base.is_empty() {
        "https://api.anthropic.com"
    } else {
        &config.litellm_params.api_base
    };
    let url = format!("{}/v1/messages", api_base.trim_end_matches('/'));

    // 发送请求
    let response = client
        .post(&url)
        .header("x-api-key", &config.litellm_params.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&anthropic_req)
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
            format!("Anthropic API 错误: {}", error_body),
        ));
    }

    // 解析响应
    let anthropic_resp: AnthropicResponse = response.json().await?;
    Ok(convert_response(anthropic_resp))
}

/// 转发流式请求到 Anthropic
pub async fn forward_request_stream(
    config: &ModelConfig,
    req: &ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>> {
    let client = get_http_client();

    // 解析模型 ID
    let (_, model_id) = parse_model_string(&config.litellm_params.model)?;

    // 转换为流式请求
    let anthropic_req = convert_request_stream(req, &model_id);

    // 构建 URL
    let api_base = if config.litellm_params.api_base.is_empty() {
        "https://api.anthropic.com"
    } else {
        &config.litellm_params.api_base
    };
    let url = format!("{}/v1/messages", api_base.trim_end_matches('/'));

    // 发送请求
    let response = client
        .post(&url)
        .header("x-api-key", &config.litellm_params.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&anthropic_req)
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
            format!("Anthropic API 错误: {}", error_body),
        ));
    }

    // 创建 SSE 转换流
    let model_id_owned = model_id.clone();
    let stream = create_anthropic_stream(response, model_id_owned);

    Ok(Box::pin(stream))
}

/// 创建 Anthropic SSE 转换流
fn create_anthropic_stream(
    response: reqwest::Response,
    model_id: String,
) -> impl Stream<Item = Result<Bytes>> + Send + Sync {
    use futures_util::StreamExt;

    // 状态变量
    let mut message_id = String::new();
    let mut buffer = String::new();

    response.bytes_stream().filter_map(move |result| {
        let output = match result {
            Ok(bytes) => {
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                process_sse_buffer(&mut buffer, &mut message_id, &model_id)
            }
            Err(e) => Some(Err(FeatherGateError::HttpError(e))),
        };
        std::future::ready(output)
    })
}

/// 处理 SSE 缓冲区，提取完整事件
fn process_sse_buffer(
    buffer: &mut String,
    message_id: &mut String,
    model_id: &str,
) -> Option<Result<Bytes>> {
    // 查找完整的 SSE 事件（以 \n\n 结尾）
    while let Some(pos) = buffer.find("\n\n") {
        let event_str = buffer[..pos].to_string();
        *buffer = buffer[pos + 2..].to_string();

        if let Some(chunk) = parse_sse_event(&event_str, message_id, model_id) {
            return Some(Ok(chunk));
        }
    }
    None
}

/// 解析单个 SSE 事件并转换为 OpenAI 格式
fn parse_sse_event(event_str: &str, message_id: &mut String, model_id: &str) -> Option<Bytes> {
    // 提取 data 行
    let mut data_line = None;
    for line in event_str.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            data_line = Some(data);
        }
    }

    let data = data_line?;

    // 解析 JSON
    let event: AnthropicEvent = serde_json::from_str(data).ok()?;

    convert_event_to_openai(event, message_id, model_id)
}

/// 将 Anthropic 事件转换为 OpenAI SSE 格式
fn convert_event_to_openai(
    event: AnthropicEvent,
    message_id: &mut String,
    model_id: &str,
) -> Option<Bytes> {
    match event {
        AnthropicEvent::MessageStart { message } => {
            *message_id = message.id;
            None // 不输出，等待内容
        }
        AnthropicEvent::ContentBlockDelta { delta, .. } => {
            if let DeltaData::TextDelta { text } = delta {
                let chunk = create_openai_chunk(message_id, model_id, Some(&text), None);
                Some(Bytes::from(chunk))
            } else {
                None
            }
        }
        AnthropicEvent::MessageDelta { delta } => {
            let finish = delta.stop_reason.map(|r| match r.as_str() {
                "end_turn" => "stop",
                "max_tokens" => "length",
                _ => "stop",
            });
            if finish.is_some() {
                let chunk = create_openai_chunk(message_id, model_id, None, finish);
                Some(Bytes::from(chunk))
            } else {
                None
            }
        }
        AnthropicEvent::MessageStop => {
            Some(Bytes::from("data: [DONE]\n\n"))
        }
        _ => None, // 忽略其他事件
    }
}

/// 创建 OpenAI 格式的 SSE chunk
fn create_openai_chunk(
    id: &str,
    model: &str,
    content: Option<&str>,
    finish_reason: Option<&str>,
) -> String {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let delta = match content {
        Some(text) => format!(r#"{{"content":"{}"}}"#, escape_json(text)),
        None => "{}".to_string(),
    };

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
fn escape_json(s: &str) -> String {
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
            model_name: "claude".to_string(),
            litellm_params: LitellmParams {
                model: "anthropic/claude-opus-4-5".to_string(),
                api_key: "sk-ant-test".to_string(),
                api_base: api_base.to_string(),
            },
        }
    }

    #[test]
    fn test_convert_request_with_system() {
        let req = ChatRequest {
            model: "claude".to_string(),
            messages: vec![
                Message::system("You are helpful"),
                Message::user("Hello"),
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
            top_p: None,
        };

        let anthropic_req = convert_request(&req, "claude-opus-4-5");

        assert_eq!(anthropic_req.model, "claude-opus-4-5");
        assert_eq!(anthropic_req.system, Some("You are helpful".to_string()));
        assert_eq!(anthropic_req.messages.len(), 1);
        assert_eq!(anthropic_req.messages[0].role, "user");
        assert_eq!(anthropic_req.max_tokens, 100);
        assert_eq!(anthropic_req.temperature, Some(0.7));
    }

    #[test]
    fn test_convert_request_without_system() {
        let req = ChatRequest {
            model: "claude".to_string(),
            messages: vec![Message::user("Hello"), Message::assistant("Hi")],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };

        let anthropic_req = convert_request(&req, "claude-opus-4-5");

        assert_eq!(anthropic_req.system, None);
        assert_eq!(anthropic_req.messages.len(), 2);
        assert_eq!(anthropic_req.max_tokens, 1024); // 默认值
    }

    #[test]
    fn test_convert_response() {
        let anthropic_resp = AnthropicResponse {
            id: "msg_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: "Hello! How can I help?".to_string(),
            }],
            model: "claude-opus-4-5".to_string(),
            stop_reason: Some("end_turn".to_string()),
            usage: AnthropicUsage {
                input_tokens: 10,
                output_tokens: 20,
            },
        };

        let openai_resp = convert_response(anthropic_resp);

        assert_eq!(openai_resp.id, "msg_123");
        assert_eq!(openai_resp.object, "chat.completion");
        assert_eq!(openai_resp.model, "claude-opus-4-5");
        assert_eq!(openai_resp.choices.len(), 1);
        assert_eq!(openai_resp.choices[0].message.content, "Hello! How can I help?");
        assert_eq!(openai_resp.choices[0].finish_reason, Some("stop".to_string()));
        assert!(openai_resp.usage.is_some());
        assert_eq!(openai_resp.usage.as_ref().unwrap().prompt_tokens, 10);
        assert_eq!(openai_resp.usage.as_ref().unwrap().completion_tokens, 20);
        assert_eq!(openai_resp.usage.as_ref().unwrap().total_tokens, 30);
    }

    #[tokio::test]
    async fn test_forward_request_success() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/v1/messages")
            .match_header("x-api-key", "sk-ant-test")
            .match_header("anthropic-version", "2023-06-01")
            .with_status(200)
            .with_body(
                r#"{
                "id": "msg_test",
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "text",
                    "text": "Hello from Claude!"
                }],
                "model": "claude-opus-4-5",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 15,
                    "output_tokens": 25
                }
            }"#,
            )
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = ChatRequest {
            model: "claude".to_string(),
            messages: vec![Message::user("Hello")],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
            top_p: None,
        };

        let result = forward_request(&config, &req).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "msg_test");
        assert_eq!(response.choices[0].message.content, "Hello from Claude!");
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 40);

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_forward_request_api_error() {
        let mut server = setup_mock_server().await;

        let mock = server
            .mock("POST", "/v1/messages")
            .with_status(400)
            .with_body(r#"{"error": {"message": "Invalid API key"}}"#)
            .create_async()
            .await;

        let config = create_test_config(&server.url());
        let req = ChatRequest {
            model: "claude".to_string(),
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
