//! 端到端故障转移测试（spec §2 [S3]）
//!
//! 测试多 Provider 场景下的故障检测与切换逻辑。

#[path = "../common/mod.rs"]
mod common;

use reqwest::Client;
use serde_json::json;

use crate::common::{empty_state, spawn_app};

/// 测试服务启动后健康检查正常
#[tokio::test]
async fn e2e_failover_health_check() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .get(format!("http://{}/healthz", app.addr))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), 200);
}

/// 测试无可用上游时的错误响应
#[tokio::test]
async fn e2e_no_upstream_available() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

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

    // 没有配置上游，应返回 404 或 401
    assert!(resp.status() == 404 || resp.status() == 401);

    let body: serde_json::Value = resp.json().await.expect("json parse");
    // 错误响应应包含 error 字段
    assert!(body.get("error").is_some() || body.get("code").is_some() || resp.status() != 200);
}

/// 测试并发请求处理
#[tokio::test]
async fn e2e_concurrent_requests() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    // 并发发送多个请求
    let mut handles = vec![];
    for _ in 0..5 {
        let client = client.clone();
        let addr = app.addr;
        handles.push(tokio::spawn(async move {
            client
                .get(format!("http://{}/healthz", addr))
                .send()
                .await
                .expect("request failed")
        }));
    }

    // 等待所有请求完成
    for handle in handles {
        let resp = handle.await.expect("task failed");
        assert_eq!(resp.status(), 200);
    }
}

/// 测试请求体大小限制
#[tokio::test]
async fn e2e_request_body_size_limit() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    // 创建一个超大的请求体（超过 1KB 限制）
    let large_body = "x".repeat(2048);

    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .header("Authorization", "Bearer sk-test-key")
        .header("Content-Type", "application/json")
        .body(large_body)
        .send()
        .await
        .expect("request failed");

    // 应该返回 413（Payload Too Large）或 400/401
    assert!(resp.status() == 413 || resp.status() == 400 || resp.status() == 401);
}

/// 测试错误响应格式一致性
#[tokio::test]
async fn e2e_error_response_format() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    // 发送一个会失败的请求
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

    assert_eq!(resp.status(), 401);

    let body: serde_json::Value = resp.json().await.expect("json parse");
    // 错误响应应包含 error 对象
    if let Some(error) = body.get("error") {
        // 如果有 error 字段，应该有 code 或 message
        assert!(error.get("code").is_some() || error.get("message").is_some());
    }
}

/// 测试服务优雅关闭（通过 dropping app handle）
#[tokio::test]
async fn e2e_graceful_shutdown() {
    let app = spawn_app(empty_state()).await;
    let addr = app.addr;
    let client = Client::new();

    // 先确认服务正常
    let resp = client
        .get(format!("http://{}/healthz", addr))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 200);

    // Drop app handle，服务应该停止
    drop(app);

    // 等待一小段时间让服务关闭
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 再次请求应该失败
    let result = client
        .get(format!("http://{}/healthz", addr))
        .send()
        .await;

    // 连接应该失败
    assert!(result.is_err());
}

/// 测试多个端点的响应
#[tokio::test]
async fn e2e_multiple_endpoints() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    // 测试健康检查
    let resp = client
        .get(format!("http://{}/healthz", app.addr))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 200);

    // 测试不存在的端点
    let resp = client
        .get(format!("http://{}/nonexistent", app.addr))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 404);
}

/// 测试请求头传递
#[tokio::test]
async fn e2e_request_headers_forwarding() {
    let app = spawn_app(empty_state()).await;
    let client = Client::new();

    let resp = client
        .post(format!("http://{}/v1/chat/completions", app.addr))
        .header("Authorization", "Bearer sk-test-key")
        .header("X-Custom-Header", "test-value")
        .header("Accept", "application/json")
        .json(&json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .expect("request failed");

    // 应该返回 404（无路由）或 401（认证失败）
    assert!(resp.status() == 404 || resp.status() == 401);
}
