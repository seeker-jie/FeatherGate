use crate::types::ChatStreamChunk;
use hyper::body::Bytes;

/// 格式化 SSE 数据块
pub fn format_sse_chunk(chunk: &ChatStreamChunk) -> String {
    let json = serde_json::to_string(chunk).unwrap_or_default();
    format!("data: {}\n\n", json)
}

/// 格式化 SSE 结束标记
pub fn format_sse_done() -> String {
    "data: [DONE]\n\n".to_string()
}

/// 将字符串转换为 SSE Bytes
pub fn to_sse_bytes(data: &str) -> Bytes {
    Bytes::from(data.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Delta, StreamChoice};

    #[test]
    fn test_format_sse_chunk() {
        let chunk = ChatStreamChunk {
            id: "chatcmpl-123".to_string(),
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

        let sse = format_sse_chunk(&chunk);
        assert!(sse.starts_with("data: "));
        assert!(sse.ends_with("\n\n"));
        assert!(sse.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_format_sse_done() {
        let done = format_sse_done();
        assert_eq!(done, "data: [DONE]\n\n");
    }

    #[test]
    fn test_to_sse_bytes() {
        let data = "test data";
        let bytes = to_sse_bytes(data);
        assert_eq!(bytes, Bytes::from("test data"));
    }

    #[test]
    fn test_sse_chunk_contains_all_fields() {
        let chunk = ChatStreamChunk {
            id: "test-id".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: Delta {
                    role: None,
                    content: Some("World".to_string()),
                },
                finish_reason: Some("stop".to_string()),
            }],
        };

        let sse = format_sse_chunk(&chunk);
        assert!(sse.contains("\"id\":\"test-id\""));
        assert!(sse.contains("\"model\":\"gpt-4\""));
        assert!(sse.contains("\"finish_reason\":\"stop\""));
    }

    #[test]
    fn test_multiple_chunks_format() {
        let chunk1 = ChatStreamChunk {
            id: "1".to_string(),
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

        let chunk2 = ChatStreamChunk {
            id: "1".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: Delta {
                    role: None,
                    content: Some(" World".to_string()),
                },
                finish_reason: None,
            }],
        };

        let sse1 = format_sse_chunk(&chunk1);
        let sse2 = format_sse_chunk(&chunk2);
        let done = format_sse_done();

        // 模拟完整的 SSE 流
        let full_stream = format!("{}{}{}", sse1, sse2, done);
        assert!(full_stream.contains("Hello"));
        assert!(full_stream.contains("World"));
        assert!(full_stream.ends_with("[DONE]\n\n"));
    }
}
