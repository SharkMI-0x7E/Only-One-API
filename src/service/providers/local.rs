//! Local Provider 实现
//!
//! 支持本地模型（Ollama / vLLM / LocalAI），兼容 OpenAI API 格式。
//! 阶段三新增（spec §2 [S3]）。

use async_trait::async_trait;
use serde_json::Value;

use crate::core::config::ProviderKind;
use crate::core::error::CoreError;
use crate::service::providers::{Provider, ProviderRequest, ProviderResponse};

/// Local Provider
///
/// 本地模型通常兼容 OpenAI API 格式，因此直接复用 OpenAI 实现。
pub struct LocalProvider;

#[async_trait]
impl Provider for LocalProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
    }

    fn transform_request(&self, req: &ProviderRequest) -> Result<Value, CoreError> {
        // 本地模型（Ollama / vLLM）兼容 OpenAI 格式，直接返回
        Ok(req.body.clone())
    }

    fn transform_response(&self, resp: &ProviderResponse) -> Result<Value, CoreError> {
        // 本地模型响应格式与 OpenAI 相同，直接返回
        if resp.is_stream {
            Ok(serde_json::from_slice(&resp.body).unwrap_or(Value::Null))
        } else {
            serde_json::from_slice(&resp.body)
                .map_err(|e| CoreError::Internal(format!("failed to parse Local response: {e}")))
        }
    }

    fn api_path(&self) -> &str {
        // 本地模型通常使用 OpenAI 兼容路径
        "/v1/chat/completions"
    }
}
