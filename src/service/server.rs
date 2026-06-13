//! service/server — axum::Router 组装 + graceful shutdown（spec §5.6）

use std::sync::Arc;

use axum::extract::Request;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::service::handler;
use crate::service::middleware::trace::request_id;
use crate::service::state::AppState;

/// 未知路径 → 结构化 404 JSON（保持 spec §5.1 错误格式）
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

/// 组装顶层 axum::Router
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handler::chat_completions))
        .route("/v1/embeddings", post(handler::embeddings))
        .route("/v1/models", get(handler::list_models))
        .route("/healthz", get(handler::healthz))
        .route("/readyz", get(handler::readyz))
        .fallback(fallback_404)
        .layer(axum::middleware::from_fn(request_id))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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
