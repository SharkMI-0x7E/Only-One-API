//! 错误响应格式（[S1] 集成测试）

mod common;

#[tokio::test]
async fn unknown_route_error_json_shape() {
    let app = common::spawn_app(common::empty_state()).await;
    let addr = app.addr;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("http://{}/no-such", addr))
        .send()
        .await
        .unwrap();
    let status = r.status();
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "route_not_found");
    assert!(body["error"]["message"].is_string());
    drop(app);
}

#[tokio::test]
async fn readyz_503_has_config_error_code() {
    let app = common::spawn_app(common::empty_state()).await;
    let addr = app.addr;
    let client = reqwest::Client::new();
    let r = client
        .get(format!("http://{}/readyz", addr))
        .send()
        .await
        .unwrap();
    let status = r.status();
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(status, 503);
    assert!(body["error"]["code"].is_string());
    drop(app);
}
