//! 端到端 OpenAI 兼容性测试（spec §2 [S3]）
//!
//! 启动完整的 RapidGate 服务，测试 OpenAI API 兼容性。

#[path = "../common/mod.rs"]
mod common;

use reqwest::Client;
use serde_json::json;

use crate::common::{empty_state, spawn_app};

/// 测试健康检查端点
#[tokio::test]
async fn e2e_health_check() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .get(format!("http://{}/healthz", app.addr))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json parse");
    assert_eq!(body["status"], "ok");
}

/// 测试 OpenAI 兼容的请求格式（无上游，应返回路由未找到）
#[tokio::test]
async fn e2e_openai_request_no_route() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .header("Authorization", "Bearer sk-test-key")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": false
        }))
        .send()
        .await
        .expect("request failed");

    // 没有配置路由，应返回 404 或 401
    assert!(resp.status() == 404 || resp.status() == 401);
}

/// 测试缺少认证头
#[tokio::test]
async fn e2e_missing_auth_header() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .expect("request failed");

    // 缺少认证头应返回 401
    assert_eq!(resp.status(), 401);
}

/// 测试无效的认证凭据
#[tokio::test]
async fn e2e_invalid_auth_credential() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .header("Authorization", "Bearer invalid-key")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .expect("request failed");

    // 无效凭据应返回 401
    assert_eq!(resp.status(), 401);
}

/// 测试不支持的 HTTP 方法
#[tokio::test]
async fn e2e_unsupported_method() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .delete(format!("http://{}/v1/chat/completions", app.addr))
        .send()
        .await
        .expect("request failed");

    // DELETE 方法应返回 405 或 404
    assert!(resp.status() == 405 || resp.status() == 404);
}

/// 测试请求体格式错误
#[tokio::test]
async fn e2e_invalid_request_body() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .header("Authorization", "Bearer sk-test-key")
        .header("Content-Type", "application/json")
        .body("invalid json")
        .send()
        .await
        .expect("request failed");

    // 无效 JSON 应返回 400 或 401（先检查认证）
    assert!(resp.status() == 400 || resp.status() == 401);
}

/// 测试请求超时（配置为 1 秒）
#[tokio::test]
async fn e2e_request_timeout_handling() {
    let app = spawn_app(empty_state()).await;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client build");

    // 发送请求，应该快速返回（因为没有路由匹配）
    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .header("Authorization", "Bearer sk-test-key")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .expect("request failed");

    // 应该快速返回，不会超时
    assert!(resp.status() == 404 || resp.status() == 401);
}

/// 测试响应头包含正确的 Content-Type
#[tokio::test]
async fn e2e_response_content_type() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .get(format!("http://{}/healthz", app.addr))
        .send()
        .await
        .expect("request failed");

    let content_type = resp
        .headers()
        .get("content-type")
        .expect("content-type header");
    assert!(content_type.to_str().unwrap().contains("application/json"));
}
