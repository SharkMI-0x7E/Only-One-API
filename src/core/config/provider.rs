//! Provider 配置模型
//!
//! 定义 LLM Provider 的类型枚举和配置 trait。
//! 阶段三新增（spec §2 [S3]）。

use serde::{Deserialize, Serialize};
use std::fmt;

/// Provider 类型枚举
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    OpenAI,
    Anthropic,
    Gemini,
    Local,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderKind::OpenAI => write!(f, "openai"),
            ProviderKind::Anthropic => write!(f, "anthropic"),
            ProviderKind::Gemini => write!(f, "gemini"),
            ProviderKind::Local => write!(f, "local"),
        }
    }
}

/// Provider 配置 trait
///
/// 所有 Provider 配置必须实现此 trait，提供请求/响应转换能力。
pub trait ProviderConfig: Send + Sync {
    /// 获取 Provider 类型
    fn kind(&self) -> ProviderKind;

    /// 获取 API base URL
    fn base_url(&self) -> &str;

    /// 获取 API Key
    fn api_key(&self) -> &str;

    /// 获取模型名称
    fn model(&self) -> &str;
}
