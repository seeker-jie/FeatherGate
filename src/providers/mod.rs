pub mod routing;
pub mod openai;
pub mod anthropic;
pub mod gemini;

use crate::config::ModelConfig;
use crate::types::{ChatRequest, ChatResponse};
use crate::Result;

/// Provider trait - 所有 provider 必须实现
#[allow(async_fn_in_trait)]
pub trait Provider {
    async fn forward_request(
        config: &ModelConfig,
        req: &ChatRequest,
    ) -> Result<ChatResponse>;
}
