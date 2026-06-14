//! 鉴权中间件（spec §5.4）
//!
//! 从请求提取 `Authorization` header → 判断类型 → 调用对应 Authenticator。
//! 校验失败返回 401 + JSON `unauthorized`，**不**区分"key 不存在" vs "key 错误"。

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// 鉴权中间件
pub async fn auth_middleware(req: Request, next: Next) -> Response {
    // 从 AppState 提取认证器（此处简化：始终通过）
    // 实际鉴权在 handler 层按 route.auth.kind 决定
    // 阶段二先实现框架，具体校验逻辑在 route 级别启用
    next.run(req).await
}

/// 从 Authorization header 提取凭据
pub fn extract_credential(req: &Request) -> Option<(AuthType, String)> {
    let auth_header = req.headers().get("authorization")?;
    let value = auth_header.to_str().ok()?;

    if let Some(token) = value.strip_prefix("Bearer ") {
        Some((AuthType::Bearer, token.trim().to_string()))
    } else if let Some(key) = value.strip_prefix("ApiKey ") {
        Some((AuthType::ApiKey, key.trim().to_string()))
    } else {
        // 无 prefix 时当作 API Key
        Some((AuthType::ApiKey, value.trim().to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    Bearer,
    ApiKey,
}

/// 构造 401 未授权响应
pub fn unauthorized_response() -> Response {
    let body = json!({
        "error": {
            "code": "unauthorized",
            "message": "invalid or missing credentials",
        }
    });
    (
        StatusCode::UNAUTHORIZED,
        [("content-type", "application/json")],
        body.to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;

    #[test]
    fn extract_bearer() {
        let req = HttpRequest::builder()
            .header("authorization", "Bearer my-token")
            .body(Body::empty())
            .unwrap();
        let (auth_type, cred) = extract_credential(&req).unwrap();
        assert_eq!(auth_type, AuthType::Bearer);
        assert_eq!(cred, "my-token");
    }

    #[test]
    fn extract_api_key() {
        let req = HttpRequest::builder()
            .header("authorization", "ApiKey sk-test-key")
            .body(Body::empty())
            .unwrap();
        let (auth_type, cred) = extract_credential(&req).unwrap();
        assert_eq!(auth_type, AuthType::ApiKey);
        assert_eq!(cred, "sk-test-key");
    }

    #[test]
    fn missing_header_returns_none() {
        let req = HttpRequest::builder().body(Body::empty()).unwrap();
        assert!(extract_credential(&req).is_none());
    }
}
