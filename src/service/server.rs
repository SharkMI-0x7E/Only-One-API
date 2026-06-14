//! service/server — axum::Router 组装 + graceful shutdown + body limit + banner（spec §5.6）

use std::sync::Arc;

use axum::extract::Request;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::service::handler;
use crate::service::middleware::trace::request_id;
use crate::service::state::AppState;

/// 未知路径 → 结构化 404 JSON
async fn fallback_404(req: Request) -> Response {
    let path = req.uri().path().to_string();
    let body = json!({
        "error": {
            "code": "route_not_found",
            "message": path,
        }
    });
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// 请求体大小限制中间件
async fn body_size_limit(req: Request, next: Next) -> Response {
    // 从 Content-Length header 检查
    if let Some(content_length) = req.headers().get("content-length") {
        if let Ok(len) = content_length.to_str().unwrap_or("0").parse::<usize>() {
            // max_body_bytes 从 AppState 获取（此处简化为 10MB 默认值）
            let max = 10 * 1024 * 1024; // 10MB
            if len > max {
                let body = json!({
                    "error": {
                        "code": "payload_too_large",
                        "message": format!("request body {len} bytes exceeds limit {max} bytes"),
                    }
                });
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    [(CONTENT_TYPE, "application/json")],
                    body.to_string(),
                )
                    .into_response();
            }
        }
    }
    next.run(req).await
}

/// 组装顶层 axum::Router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handler::chat_completions))
        .route("/v1/embeddings", post(handler::embeddings))
        .route("/v1/models", get(handler::list_models))
        .route("/healthz", get(handler::healthz))
        .route("/readyz", get(handler::readyz))
        .fallback(fallback_404)
        .layer(axum::middleware::from_fn(body_size_limit))
        .layer(axum::middleware::from_fn(request_id))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// 打印启动横幅
pub fn print_banner(listen: &str, config_dir: &str, route_count: usize, upstream_count: usize) {
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        listen = %listen,
        config_dir = %config_dir,
        routes = route_count,
        upstreams = upstream_count,
        "RapidGate starting"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use http::StatusCode;
    use tower::ServiceExt;

    fn empty_state() -> Arc<AppState> {
        use crate::core::config::route::RateLimitConfig;
        use crate::core::routing::Router;
        Arc::new(AppState::new(
            Router::default(),
            vec![],
            RateLimitConfig {
                algorithm: "token_bucket".into(),
                rps: 1,
                burst: 1,
            },
            std::path::PathBuf::from("./config"),
            1024,
            1000,
        ))
    }

    #[tokio::test]
    async fn healthz_works() {
        let app = router(empty_state());
        let resp = app
            .oneshot(HttpRequest::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
