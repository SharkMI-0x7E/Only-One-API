//! Plugin trait 定义
//!
//! 阶段三新增（spec §2 [S3]）。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 插件错误
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("plugin permission denied: {0}")]
    PermissionDenied(String),
}

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

/// 请求上下文（插件可读写）
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub metadata: HashMap<String, String>,
}

/// 代理上下文（插件可读写）
#[derive(Debug, Clone)]
pub struct ProxyContext {
    pub upstream_url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub metadata: HashMap<String, String>,
}

/// 错误上下文（插件只读）
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error_code: String,
    pub error_message: String,
    pub request_context: RequestContext,
}

/// 插件 trait
///
/// 所有插件必须实现此 trait。插件可以在请求生命周期的不同阶段执行逻辑。
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 获取插件元数据
    fn metadata(&self) -> PluginMetadata;

    /// 请求到达时调用（在鉴权、限流之前）
    async fn on_request(&self, _ctx: &mut RequestContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// 代理转发前调用
    async fn before_proxy(&self, _ctx: &mut ProxyContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// 代理转发后调用
    async fn after_proxy(&self, _ctx: &mut ProxyContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// 发生错误时调用
    async fn on_error(&self, _ctx: &ErrorContext) -> Result<(), PluginError> {
        Ok(())
    }
}
