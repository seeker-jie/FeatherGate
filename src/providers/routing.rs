use crate::config::{parse_model_string, Config};
use crate::error::FeatherGateError;
use crate::providers::{anthropic, gemini, openai};
use crate::types::{ChatRequest, ChatResponse};
use crate::Result;
use futures_util::Stream;
use hyper::body::Bytes;
use std::pin::Pin;
use std::sync::Arc;

/// 路由请求到正确的 provider
pub async fn route_request(
    config: Arc<Config>,
    req: ChatRequest,
) -> Result<ChatResponse> {
    // 查找模型配置
    let model_config = config
        .find_model(&req.model)
        .ok_or_else(|| FeatherGateError::ModelNotFound(req.model.clone()))?;

    // 解析 provider
    let (provider, _model_id) = parse_model_string(&model_config.litellm_params.model)?;

    // 路由到对应 provider
    match provider.as_str() {
        "openai" => openai::forward_request(model_config, &req).await,
        "anthropic" => anthropic::forward_request(model_config, &req).await,
        "gemini" => gemini::forward_request(model_config, &req).await,
        _ => Err(FeatherGateError::UnsupportedProvider(provider)),
    }
}

/// 路由流式请求到正确的 provider
pub async fn route_request_stream(
    config: Arc<Config>,
    req: ChatRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>> {
    // 查找模型配置
    let model_config = config
        .find_model(&req.model)
        .ok_or_else(|| FeatherGateError::ModelNotFound(req.model.clone()))?;

    // 解析 provider
    let (provider, _model_id) = parse_model_string(&model_config.litellm_params.model)?;

    // 路由到对应 provider（支持所有提供商流式）
    match provider.as_str() {
        "openai" => openai::forward_request_stream(model_config, &req).await,
        "anthropic" => anthropic::forward_request_stream(model_config, &req).await,
        "gemini" => gemini::forward_request_stream(model_config, &req).await,
        _ => Err(FeatherGateError::UnsupportedProvider(provider)),
    }
}

/// 根据模型字符串判断 provider
pub fn determine_provider(model: &str) -> Result<String> {
    let (provider, _) = parse_model_string(model)?;
    Ok(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LitellmParams, ModelConfig};
    use crate::types::Message;

    fn create_test_config() -> Config {
        Config {
            model_list: vec![
                ModelConfig {
                    model_name: "gpt-4".to_string(),
                    litellm_params: LitellmParams {
                        model: "openai/gpt-4".to_string(),
                        api_key: "sk-test".to_string(),
                        api_base: "https://api.openai.com".to_string(),
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
        }
    }

    #[test]
    fn test_determine_provider() {
        assert_eq!(determine_provider("openai/gpt-4").unwrap(), "openai");
        assert_eq!(
            determine_provider("anthropic/claude-opus-4-5").unwrap(),
            "anthropic"
        );
        assert_eq!(determine_provider("gemini/gemini-pro").unwrap(), "gemini");
    }

    #[test]
    fn test_determine_provider_invalid() {
        assert!(determine_provider("invalid").is_err());
    }

    #[tokio::test]
    async fn test_route_request_model_not_found() {
        let config = Arc::new(create_test_config());
        let req = ChatRequest {
            model: "non-existent".to_string(),
            messages: vec![Message::user("test")],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };

        let result = route_request(config, req).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FeatherGateError::ModelNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_route_request_unsupported_provider() {
        let config = Arc::new(Config {
            model_list: vec![ModelConfig {
                model_name: "unknown".to_string(),
                litellm_params: LitellmParams {
                    model: "unknown-provider/model".to_string(),
                    api_key: "test".to_string(),
                    api_base: String::new(),
                },
            }],
        });

        let req = ChatRequest {
            model: "unknown".to_string(),
            messages: vec![Message::user("test")],
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
        };

        let result = route_request(config, req).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FeatherGateError::UnsupportedProvider(_)
        ));
    }
}
