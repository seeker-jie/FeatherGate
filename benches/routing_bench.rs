use criterion::{black_box, criterion_group, criterion_main, Criterion};
use feathergate::config::parse_model_string;

fn bench_parse_model_string(c: &mut Criterion) {
    c.bench_function("parse_model_string_openai", |b| {
        b.iter(|| {
            parse_model_string(black_box("openai/gpt-4"))
        })
    });

    c.bench_function("parse_model_string_anthropic", |b| {
        b.iter(|| {
            parse_model_string(black_box("anthropic/claude-opus-4-5"))
        })
    });

    c.bench_function("parse_model_string_gemini", |b| {
        b.iter(|| {
            parse_model_string(black_box("gemini/gemini-pro"))
        })
    });
}

fn bench_config_find_model(c: &mut Criterion) {
    use feathergate::config::{Config, LitellmParams, ModelConfig};

    let config = Config {
        model_list: vec![
            ModelConfig {
                model_name: "gpt-4".to_string(),
                litellm_params: LitellmParams {
                    model: "openai/gpt-4".to_string(),
                    api_key: "sk-test".to_string(),
                    api_base: "https://api.openai.com/v1".to_string(),
                },
            },
            ModelConfig {
                model_name: "claude".to_string(),
                litellm_params: LitellmParams {
                    model: "anthropic/claude-opus-4-5".to_string(),
                    api_key: "sk-ant-test".to_string(),
                    api_base: "https://api.anthropic.com".to_string(),
                },
            },
            ModelConfig {
                model_name: "gemini".to_string(),
                litellm_params: LitellmParams {
                    model: "gemini/gemini-pro".to_string(),
                    api_key: "AIza-test".to_string(),
                    api_base: "https://generativelanguage.googleapis.com".to_string(),
                },
            },
        ],
    };

    c.bench_function("find_model_first", |b| {
        b.iter(|| {
            config.find_model(black_box("gpt-4"))
        })
    });

    c.bench_function("find_model_middle", |b| {
        b.iter(|| {
            config.find_model(black_box("claude"))
        })
    });

    c.bench_function("find_model_last", |b| {
        b.iter(|| {
            config.find_model(black_box("gemini"))
        })
    });

    c.bench_function("find_model_not_found", |b| {
        b.iter(|| {
            config.find_model(black_box("non-existent"))
        })
    });
}

fn bench_message_constructors(c: &mut Criterion) {
    use feathergate::types::Message;

    c.bench_function("message_user", |b| {
        b.iter(|| {
            Message::user(black_box("Hello"))
        })
    });

    c.bench_function("message_assistant", |b| {
        b.iter(|| {
            Message::assistant(black_box("Hi there"))
        })
    });

    c.bench_function("message_system", |b| {
        b.iter(|| {
            Message::system(black_box("You are helpful"))
        })
    });
}

fn bench_sse_formatting(c: &mut Criterion) {
    use feathergate::server::streaming::{format_sse_chunk, format_sse_done};
    use feathergate::types::{ChatStreamChunk, Delta, StreamChoice};

    let chunk = ChatStreamChunk {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion.chunk".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![StreamChoice {
            index: 0,
            delta: Delta {
                role: Some("assistant".to_string()),
                content: Some("Hello, how can I help you today?".to_string()),
            },
            finish_reason: None,
        }],
    };

    c.bench_function("format_sse_chunk", |b| {
        b.iter(|| {
            format_sse_chunk(black_box(&chunk))
        })
    });

    c.bench_function("format_sse_done", |b| {
        b.iter(|| {
            format_sse_done()
        })
    });
}

fn bench_serialization(c: &mut Criterion) {
    use feathergate::types::{ChatRequest, ChatResponse, Message};

    let request = ChatRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello!"),
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream: Some(false),
        top_p: Some(1.0),
    };

    c.bench_function("serialize_chat_request", |b| {
        b.iter(|| {
            serde_json::to_string(black_box(&request))
        })
    });

    let response = ChatResponse::simple("gpt-4", "Hello! How can I help you today?");

    c.bench_function("serialize_chat_response", |b| {
        b.iter(|| {
            serde_json::to_string(black_box(&response))
        })
    });

    let request_json = serde_json::to_string(&request).unwrap();

    c.bench_function("deserialize_chat_request", |b| {
        b.iter(|| {
            serde_json::from_str::<ChatRequest>(black_box(&request_json))
        })
    });
}

criterion_group!(
    benches,
    bench_parse_model_string,
    bench_config_find_model,
    bench_message_constructors,
    bench_sse_formatting,
    bench_serialization
);
criterion_main!(benches);
