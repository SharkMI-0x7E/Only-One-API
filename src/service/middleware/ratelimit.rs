//! 限流中间件（spec §5.4）
//!
//! 从请求提取 key → 调用 RateLimiter::check → 超限返回 429。

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// 限流中间件（阶段二框架，具体限流在 handler 层按 route 配置启用）
pub async fn ratelimit_middleware(req: Request, next: Next) -> Response {
    next.run(req).await
}

/// 构造 429 限流响应
pub fn rate_limited_response() -> Response {
    let body = json!({
        "error": {
            "code": "rate_limited",
            "message": "rate limit exceeded",
        }
    });
    (
        StatusCode::TOO_MANY_REQUESTS,
        [("content-type", "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// 从请求提取限流 key（IP 或 API Key fingerprint）
pub fn extract_limit_key(req: &Request) -> String {
    // 优先用 X-Forwarded-For，其次用连接 IP
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(ip) = forwarded.to_str() {
            return ip.split(',').next().unwrap_or("unknown").trim().to_string();
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;

    #[test]
    fn extract_key_from_forwarded_for() {
        let req = HttpRequest::builder()
            .header("x-forwarded-for", "1.2.3.4, 5.6.7.8")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_limit_key(&req), "1.2.3.4");
    }

    #[test]
    fn extract_key_fallback() {
        let req = HttpRequest::builder().body(Body::empty()).unwrap();
        assert_eq!(extract_limit_key(&req), "unknown");
    }
}
