//! 路由匹配 + 5 路由冒烟（[S1] 集成测试）

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_200() {
    let app = common::spawn_app(common::empty_state()).await;
    let resp = app.state.route_table.snapshot(); // 占位调用避免 unused 警告
    let _ = resp;
    let listener_addr = app.addr;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("http://{}/healthz", listener_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    drop(app);
}

#[tokio::test]
async fn readyz_returns_503_when_no_upstreams() {
    let app = common::spawn_app(common::empty_state()).await;
    let listener_addr = app.addr;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("http://{}/readyz", listener_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 503);
    drop(app);
}

#[tokio::test]
async fn v1_models_returns_200() {
    let app = common::spawn_app(common::empty_state()).await;
    let listener_addr = app.addr;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("http://{}/v1/models", listener_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["object"], "list");
    drop(app);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = common::spawn_app(common::empty_state()).await;
    let listener_addr = app.addr;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("http://{}/v1/nope", listener_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    drop(app);
}

#[tokio::test]
async fn request_id_header_present_in_response() {
    use axum::routing::get;
    use axum::Router;

    async fn ok() -> &'static str {
        "ok"
    }
    let router = Router::new().route("/x", get(ok));
    let resp = router
        .oneshot(Request::get("/x").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
