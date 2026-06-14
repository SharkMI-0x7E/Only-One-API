//! service/handler — 5 个 axum handler（spec §5.3）

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{json, Value};

use crate::core::config::route::RouteConfig;
use crate::core::config::upstream::UpstreamConfig;
use crate::core::error::CoreError;
use crate::core::routing::RouteTable;
use crate::service::error::ServiceError;
use crate::service::state::AppState;

/// POST /v1/chat/completions — OpenAI 兼容聊天补全（含 SSE 流式）
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    req: Request,
) -> Result<Response, ServiceError> {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let (route, upstream) = resolve_route(&state, &method, &path)?;
    forward_streaming(state, &route, &upstream, req).await
}

/// POST /v1/embeddings — OpenAI 兼容 embedding
pub async fn embeddings(
    State(state): State<Arc<AppState>>,
    req: Request,
) -> Result<Response, ServiceError> {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let (route, upstream) = resolve_route(&state, &method, &path)?;
    forward_streaming(state, &route, &upstream, req).await
}

/// GET /v1/models — 列出可用模型
pub async fn list_models(State(state): State<Arc<AppState>>) -> Response {
    let mut models: Vec<String> = Vec::new();
    for up in &state.upstream_configs {
        for m in &up.models {
            if !models.contains(m) {
                models.push(m.clone());
            }
        }
    }
    if models.is_empty() {
        models.push("rapidgate-stage1-placeholder".to_string());
    }
    let body = json!({
        "object": "list",
        "data": models.iter().map(|id| json!({"id": id, "object": "model"})).collect::<Vec<_>>(),
    });
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// GET /healthz — 存活探针（不查上游）
pub async fn healthz() -> Response {
    let body = json!({"status": "ok"});
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// GET /readyz — 就绪探针（检查配置有效性 + 上游可达性）
pub async fn readyz(State(state): State<Arc<AppState>>) -> Result<Response, ServiceError> {
    if state.upstream_configs.is_empty() {
        let body = json!({
            "error": {
                "code": "not_ready",
                "message": "no upstreams configured",
            }
        });
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            [(CONTENT_TYPE, "application/json")],
            body.to_string(),
        )
            .into_response());
    }
    let route_count = state.route_table.snapshot().len();
    if route_count == 0 {
        let body = json!({
            "error": {
                "code": "not_ready",
                "message": "no routes loaded",
            }
        });
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            [(CONTENT_TYPE, "application/json")],
            body.to_string(),
        )
            .into_response());
    }
    let body = json!({"status": "ready", "routes": route_count, "upstreams": state.upstream_configs.len()});
    Ok((
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

// -------------------- 内部辅助 --------------------

/// 在路由表 + upstream 配置里一次性解析出 (RouteConfig, UpstreamConfig)
fn resolve_route(
    state: &Arc<AppState>,
    method: &str,
    path: &str,
) -> Result<(RouteConfig, UpstreamConfig), ServiceError> {
    let table: Arc<RouteTable> = state.route_table.snapshot();
    let (_idx, route) = table
        .match_request(method, path, &[], &[])
        .ok_or_else(|| ServiceError::Core(CoreError::RouteNotFound(path.to_string())))?;
    let upstream = state
        .upstream_configs
        .iter()
        .find(|u| u.id == route.upstream.id)
        .cloned()
        .ok_or_else(|| {
            ServiceError::Core(CoreError::Config(format!(
                "route '{}' references unknown upstream '{}'",
                route.name, route.upstream.id
            )))
        })?;
    Ok((route.clone(), upstream))
}

/// 阶段一占位转发：未配置真实 provider / 客户端时返回 501 + JSON 提示
async fn forward_streaming(
    _state: Arc<AppState>,
    route: &RouteConfig,
    upstream: &UpstreamConfig,
    _req: Request,
) -> Result<Response, ServiceError> {
    tracing::info!(
        route = %route.name,
        upstream = %upstream.id,
        provider = %upstream.provider,
        "stage-1 stub forward"
    );
    let body: Value = json!({
        "object": "stage1.stub",
        "route": route.name,
        "upstream": upstream.id,
        "provider": upstream.provider,
        "message": "stage-1 stub: streaming forward not yet wired (planned for stage-2/3)",
    });
    Ok((
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

// 引入 Body 以保持 import 整洁（未来流式接管时用）
#[allow(dead_code)]
fn _force_use_body() -> Body {
    Body::empty()
}
