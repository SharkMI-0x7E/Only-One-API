//! Admin API authentication middleware (spec §5.7)
//!
//! Reads `RGD_ADMIN_TOKEN` from the environment and validates the
//! `Authorization: Bearer <token>` header using constant-time comparison.

use axum::extract::Request;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use subtle::ConstantTimeEq;

/// Admin API authentication middleware.
///
/// Rejects the request with 401 if:
/// - `RGD_ADMIN_TOKEN` is not set or empty
/// - The `Authorization` header is missing or malformed
/// - The provided token does not match the expected token (constant-time comparison)
pub async fn admin_auth(req: Request, next: Next) -> Response {
    let expected = match std::env::var("RGD_ADMIN_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            tracing::error!("RGD_ADMIN_TOKEN not set, rejecting admin request");
            return unauthorized_response();
        }
    };

    let provided = match extract_bearer(&req) {
        Some(t) => t,
        None => return unauthorized_response(),
    };

    // Constant-time comparison to avoid timing attacks
    let provided_bytes = provided.as_bytes();
    let expected_bytes = expected.as_bytes();

    // If lengths differ, ct_eq would panic or give wrong results,
    // so we compare lengths first (length leak is acceptable for tokens).
    if provided_bytes.len() != expected_bytes.len() {
        return unauthorized_response();
    }

    if bool::from(provided_bytes.ct_eq(expected_bytes)) {
        next.run(req).await
    } else {
        unauthorized_response()
    }
}

/// Extract the bearer token from the `Authorization` header.
fn extract_bearer(req: &Request) -> Option<String> {
    let header = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Build a 401 JSON error response.
fn unauthorized_response() -> Response {
    let body = json!({
        "error": {
            "code": "unauthorized",
            "message": "invalid or missing admin token"
        }
    });
    (
        StatusCode::UNAUTHORIZED,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
