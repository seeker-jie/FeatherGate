use crate::config::Config;
use crate::metrics;
use crate::providers::routing;
use crate::types::ChatRequest;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::{Method, Request, Response, StatusCode};
use serde_json::json;
use std::sync::Arc;

// 统一的 Body 类型，可以处理普通响应和流式响应
type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BoxBody = http_body_util::combinators::BoxBody<Bytes, BoxError>;

/// 处理 HTTP 请求的主路由
pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    config: Arc<Config>,
) -> Result<Response<BoxBody>, BoxError> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/health") => Ok(health_check()),
        (&Method::GET, "/v1/models") => Ok(list_models(config)),
        (&Method::GET, "/metrics") => Ok(metrics_endpoint()),
        (&Method::POST, "/v1/chat/completions") => chat_completions(req, config).await,
        _ => Ok(not_found()),
    }
}

/// 健康检查端点
fn health_check() -> Response<BoxBody> {
    let body = json!({
        "status": "ok",
        "service": "feathergate"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(
            Full::new(Bytes::from(body.to_string()))
                .map_err(|e| Box::new(e) as BoxError)
                .boxed(),
        )
        .unwrap()
}

/// 列出可用模型
fn list_models(config: Arc<Config>) -> Response<BoxBody> {
    let models: Vec<_> = config
        .model_list
        .iter()
        .map(|m| {
            json!({
                "id": m.model_name,
                "object": "model",
                "owned_by": "feathergate"
            })
        })
        .collect();

    let body = json!({
        "object": "list",
        "data": models
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(
            Full::new(Bytes::from(body.to_string()))
                .map_err(|e| Box::new(e) as BoxError)
                .boxed(),
        )
        .unwrap()
}

/// 指标端点
fn metrics_endpoint() -> Response<BoxBody> {
    let metrics = metrics::global_metrics();
    let body = metrics.export_prometheus();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(
            Full::new(Bytes::from(body))
                .map_err(|e| Box::new(e) as BoxError)
                .boxed(),
        )
        .unwrap()
}

/// 聊天完成端点
async fn chat_completions(
    req: Request<hyper::body::Incoming>,
    config: Arc<Config>,
) -> Result<Response<BoxBody>, BoxError> {
    let metrics = metrics::global_metrics();

    // 读取请求体
    let whole_body = req.collect().await?.to_bytes();
    let chat_req: ChatRequest = serde_json::from_slice(&whole_body)?;

    // 验证请求参数
    if let Err(e) = chat_req.validate() {
        let error_body = json!({
            "error": {
                "message": e,
                "type": "invalid_request_error"
            }
        });
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(
                Full::new(Bytes::from(error_body.to_string()))
                    .map_err(|e| Box::new(e) as BoxError)
                    .boxed(),
            )
            .unwrap());
    }

    // 检查是否为流式请求
    if chat_req.stream == Some(true) {
        return chat_completions_stream(chat_req, config).await;
    }

    // 路由请求
    match routing::route_request(config, chat_req).await {
        Ok(response) => {
            metrics.record_success();
            let body = serde_json::to_string(&response)?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(
                    Full::new(Bytes::from(body))
                        .map_err(|e| Box::new(e) as BoxError)
                        .boxed(),
                )
                .unwrap())
        }
        Err(e) => {
            metrics.record_failure();
            let error_body = json!({
                "error": {
                    "message": e.to_string(),
                    "type": "feathergate_error"
                }
            });

            let status = match e {
                crate::FeatherGateError::ModelNotFound(_) => StatusCode::NOT_FOUND,
                crate::FeatherGateError::UnsupportedProvider(_) => StatusCode::BAD_REQUEST,
                crate::FeatherGateError::UpstreamError { status, .. } => {
                    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(
                    Full::new(Bytes::from(error_body.to_string()))
                        .map_err(|e| Box::new(e) as BoxError)
                        .boxed(),
                )
                .unwrap())
        }
    }
}

/// 流式聊天完成端点
async fn chat_completions_stream(
    chat_req: ChatRequest,
    config: Arc<Config>,
) -> Result<Response<BoxBody>, BoxError> {
    let metrics = metrics::global_metrics();

    // 路由流式请求
    match routing::route_request_stream(config, chat_req).await {
        Ok(stream) => {
            metrics.record_success();

            // 将字节流转换为 Frame 流
            use futures_util::StreamExt;
            let frame_stream = stream.map(|result| {
                result.map(Frame::data).map_err(|e| Box::new(e) as BoxError)
            });

            // 创建 StreamBody 并转换为 BoxBody
            let body = StreamBody::new(frame_stream);
            let boxed_body = BodyExt::boxed(body);

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .header("X-Accel-Buffering", "no") // 禁用 Nginx 缓冲
                .body(boxed_body)
                .unwrap())
        }
        Err(e) => {
            metrics.record_failure();

            // 返回 SSE 格式的错误消息
            let error_data = json!({
                "error": {
                    "message": e.to_string(),
                    "type": "feathergate_error"
                }
            });
            let sse_error = format!("data: {}\n\ndata: [DONE]\n\n", error_data);

            Ok(Response::builder()
                .status(StatusCode::OK)  // SSE 需要 200 状态码
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .body(
                    Full::new(Bytes::from(sse_error))
                        .map_err(|e| Box::new(e) as BoxError)
                        .boxed(),
                )
                .unwrap())
        }
    }
}

/// 404 响应
fn not_found() -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(
            Full::new(Bytes::from("Not Found"))
                .map_err(|e| Box::new(e) as BoxError)
                .boxed(),
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LitellmParams, ModelConfig};

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
            ],
        }
    }

    #[test]
    fn test_health_check() {
        let response = health_check();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_list_models() {
        let config = Arc::new(create_test_config());
        let response = list_models(config);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_not_found() {
        let response = not_found();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
