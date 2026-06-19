//! Panic recovery middleware (spec §5.4)
//!
//! Catches panics from downstream handlers and returns a 500 JSON error response.
//! Logs panic info via tracing and prevents process crash.

use std::panic::AssertUnwindSafe;

use axum::extract::Request;
use axum::http::{HeaderName, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use futures_util::FutureExt;
use serde_json::json;

const HEADER_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Recovery middleware that catches panics and returns 500 JSON error
pub async fn recovery(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(&HEADER_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let result = AssertUnwindSafe(next.run(req)).catch_unwind().await;

    match result {
        Ok(response) => response,
        Err(panic_info) => {
            let panic_msg = extract_panic_message(&panic_info);

            tracing::error!(
                request_id = %request_id,
                panic = %panic_msg,
                "handler panicked"
            );

            let body = json!({
                "error": {
                    "code": "internal_error",
                    "message": "internal server error",
                    "request_id": request_id,
                }
            });

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "application/json")],
                body.to_string(),
            )
                .into_response()
        }
    }
}

/// Extract human-readable message from panic payload
fn extract_panic_message(panic_info: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic_info.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
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
    async fn normal_request_passes_through() {
        async fn ok_handler() -> &'static str {
            "ok"
        }
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(from_fn(recovery));

        let resp = app
            .oneshot(HttpRequest::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn panicking_handler_returns_500() {
        async fn panic_handler() -> &'static str {
            panic!("test panic message");
        }
        let app = Router::new()
            .route("/", get(panic_handler))
            .layer(from_fn(recovery));

        let resp = app
            .oneshot(HttpRequest::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(v["error"]["code"], "internal_error");
        assert_eq!(v["error"]["message"], "internal server error");
        assert!(v["error"]["request_id"].is_string());
    }

    #[tokio::test]
    async fn preserves_request_id_in_error() {
        async fn panic_handler() -> &'static str {
            panic!("boom");
        }
        let app = Router::new()
            .route("/", get(panic_handler))
            .layer(from_fn(recovery));

        let request_id = "ABCDEF1234567890ABCDEF1234567890";
        let resp = app
            .oneshot(
                HttpRequest::get("/")
                    .header(&HEADER_REQUEST_ID, request_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(v["error"]["request_id"], request_id);
    }
}
