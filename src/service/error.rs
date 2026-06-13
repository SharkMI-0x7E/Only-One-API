//! ServiceError + IntoResponse（spec §5.1）
//!
//! 整个网关与 HTTP 协议的唯一耦合点。
//! 输出 JSON：`{ "error": { "code": "...", "message": "..." } }`

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde_json::json;
use thiserror::Error;

use crate::core::error::CoreError;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("upstream http error: status={status}")]
    Upstream { status: u16, body: Bytes },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config load error: {0}")]
    ConfigLoad(String),
}

impl ServiceError {
    fn code(&self) -> &'static str {
        match self {
            ServiceError::Core(CoreError::RouteNotFound(_)) => "route_not_found",
            ServiceError::Core(CoreError::Auth(_)) => "unauthorized",
            ServiceError::Core(CoreError::RateLimited) => "rate_limited",
            ServiceError::Core(CoreError::BreakerOpen(_)) => "breaker_open",
            ServiceError::Core(CoreError::UpstreamUnreachable(_)) => "upstream_unreachable",
            ServiceError::Core(CoreError::UpstreamTimeout(_)) => "upstream_timeout",
            ServiceError::Core(CoreError::BadRequest(_)) => "bad_request",
            ServiceError::Core(CoreError::Config(_)) => "config_error",
            ServiceError::Core(CoreError::Internal(_)) => "internal",
            ServiceError::Upstream { .. } => "upstream_error",
            ServiceError::Io(_) => "io_error",
            ServiceError::ConfigLoad(_) => "config_load_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            ServiceError::Core(CoreError::RouteNotFound(_)) => StatusCode::NOT_FOUND,
            ServiceError::Core(CoreError::Auth(_)) => StatusCode::UNAUTHORIZED,
            ServiceError::Core(CoreError::RateLimited) => StatusCode::TOO_MANY_REQUESTS,
            ServiceError::Core(CoreError::BreakerOpen(_)) => StatusCode::SERVICE_UNAVAILABLE,
            ServiceError::Core(CoreError::UpstreamUnreachable(_)) => StatusCode::BAD_GATEWAY,
            ServiceError::Core(CoreError::UpstreamTimeout(_)) => StatusCode::GATEWAY_TIMEOUT,
            ServiceError::Core(CoreError::BadRequest(_)) => StatusCode::BAD_REQUEST,
            // 阶段一：配置/启动期错误映射到 503，体现"暂时不可用"语义
            ServiceError::Core(CoreError::Config(_)) => StatusCode::SERVICE_UNAVAILABLE,
            ServiceError::Upstream { status, .. } => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            ServiceError::ConfigLoad(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        let body = json!({
            "error": {
                "code": code,
                "message": self.to_string(),
            }
        });
        (
            status,
            [("content-type", "application/json")],
            body.to_string(),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request;
    use tower::ServiceExt;

    fn router() -> axum::Router {
        use axum::routing::get;
        async fn err_h() -> Result<Response, ServiceError> {
            Err(ServiceError::Core(CoreError::RouteNotFound("/x".into())))
        }
        axum::Router::new().route("/err", get(err_h))
    }

    #[tokio::test]
    async fn error_response_format() {
        let app = router();
        let resp = app
            .oneshot(
                Request::get("/err")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"]["code"], "route_not_found");
    }
}
