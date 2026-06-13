//! 核心错误类型（spec §4.1）
//!
//! 9 个变体，覆盖配置、路由、鉴权、限流、熔断、上游、bad request、内部错误。
//! `CoreError` **不**实现 `axum::response::IntoResponse`，
//! 与 HTTP 协议的唯一耦合点在 `service::error::ServiceError`。

use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("config error: {0}")]
    Config(String),

    #[error("route not found: {0}")]
    RouteNotFound(String),

    #[error("auth failed: {0}")]
    Auth(String),

    #[error("rate limit exceeded")]
    RateLimited,

    #[error("circuit breaker open: {0}")]
    BreakerOpen(String),

    #[error("upstream unreachable: {0}")]
    UpstreamUnreachable(String),

    #[error("upstream timeout after {0:?}")]
    UpstreamTimeout(Duration),

    #[error("invalid request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        CoreError::Internal(format!("io: {err}"))
    }
}

impl From<serde_yaml::Error> for CoreError {
    fn from(err: serde_yaml::Error) -> Self {
        CoreError::Config(format!("yaml: {err}"))
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        CoreError::Internal(format!("json: {err}"))
    }
}
