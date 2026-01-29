use serde::{Deserialize, Serialize};

/// OpenAI 兼容的聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

impl ChatRequest {
    /// 验证请求参数范围
    pub fn validate(&self) -> Result<(), String> {
        // 验证 temperature (0.0 - 2.0)
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(format!(
                    "temperature 必须在 0.0 到 2.0 之间，当前值: {}",
                    temp
                ));
            }
        }

        // 验证 top_p (0.0 - 1.0)
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(format!(
                    "top_p 必须在 0.0 到 1.0 之间，当前值: {}",
                    top_p
                ));
            }
        }

        // 验证 messages 非空
        if self.messages.is_empty() {
            return Err("messages 不能为空".to_string());
        }

        Ok(())
    }
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    /// 创建用户消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    /// 创建系统消息
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// OpenAI 兼容的聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// 响应选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl ChatResponse {
    /// 创建简单的响应
    pub fn simple(model: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model: model.into(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant(content),
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        }
    }
}

/// 流式响应数据块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

/// 流式响应选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

/// 流式响应增量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Hi there");

        let system_msg = Message::system("You are helpful");
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content, "You are helpful");
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("test")],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: None,
            top_p: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"temperature\":0.7"));
        // stream 为 None 应该被跳过
        assert!(!json.contains("stream"));
    }

    #[test]
    fn test_chat_request_deserialization() {
        let json = r#"{
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.8
        }"#;

        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.temperature, Some(0.8));
        assert_eq!(req.max_tokens, None);
    }

    #[test]
    fn test_chat_response_simple() {
        let resp = ChatResponse::simple("gpt-4", "Hello!");
        assert_eq!(resp.object, "chat.completion");
        assert_eq!(resp.model, "gpt-4");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert_eq!(resp.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_chat_response_serialization() {
        let resp = ChatResponse {
            id: "test-id".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("Response"),
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"id\":\"test-id\""));
        assert!(json.contains("\"prompt_tokens\":10"));
    }

    #[test]
    fn test_stream_chunk_serialization() {
        let chunk = ChatStreamChunk {
            id: "chunk-1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: Delta {
                    role: Some("assistant".to_string()),
                    content: Some("Hello".to_string()),
                },
                finish_reason: None,
            }],
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_validate_temperature_valid() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("test")],
            temperature: Some(1.0),
            max_tokens: None,
            stream: None,
            top_p: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_validate_temperature_invalid() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("test")],
            temperature: Some(3.0),
            max_tokens: None,
            stream: None,
            top_p: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_top_p_invalid() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("test")],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: Some(1.5),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_empty_messages() {
        let req = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };
        assert!(req.validate().is_err());
    }
}
