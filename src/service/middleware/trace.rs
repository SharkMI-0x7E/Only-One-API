//! X-Request-Id 中间件（spec §5.4）
//!
//! 优先级最高（最外层）：
//! - 提取 `X-Request-Id` Header；缺失则生成新 TraceId
//! - 写入 tracing span 字段 `request_id`
//! - 响应里回写 `X-Request-Id` Header

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

use crate::core::observability::TraceId;

const HEADER: HeaderName = HeaderName::from_static("x-request-id");

pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(&HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(TraceId::from_hex)
        .unwrap_or_else(TraceId::new);

    let header_val =
        HeaderValue::from_str(id.as_str()).unwrap_or_else(|_| HeaderValue::from_static("invalid"));

    // 注入回 request 头，方便下游 handler / span 读取
    req.headers_mut().insert(HEADER.clone(), header_val.clone());

    let span = tracing::info_span!("request", request_id = %id);
    let _enter = span.enter();
    tracing::info!(method = %req.method(), path = %req.uri().path(), "request started");

    let mut resp = next.run(req).await;
    resp.headers_mut().insert(HEADER, header_val);
    tracing::info!(status = %resp.status(), "request handled");
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn propagates_request_id() {
        async fn ok() -> &'static str {
            "ok"
        }
        let app = Router::new().route("/", get(ok)).layer(from_fn(request_id));

        let resp = app
            .oneshot(HttpRequest::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(resp.headers().contains_key(&HEADER));
    }

    #[tokio::test]
    async fn honors_inbound_request_id() {
        async fn echo_id(req: axum::extract::Request) -> String {
            req.headers()
                .get(&HEADER)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        }
        let app = Router::new()
            .route("/", get(echo_id))
            .layer(from_fn(request_id));

        let provided = "ABCDEF1234567890ABCDEF1234567890";
        let resp = app
            .oneshot(
                HttpRequest::get("/")
                    .header(&HEADER, provided)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let echoed = String::from_utf8_lossy(&body).trim_matches('"').to_string();
        assert_eq!(echoed, provided);
    }
}
