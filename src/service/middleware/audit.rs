//! 审计日志中间件（spec §5.4）
//!
//! 请求完成后收集 route_id / latency_ms / status，写入审计日志。

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// 审计中间件：记录请求耗时
pub async fn audit_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let resp = next.run(req).await;

    let latency_ms = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();

    tracing::info!(
        method = %method,
        path = %path,
        status = status,
        latency_ms = latency_ms,
        "audit"
    );

    resp
}
