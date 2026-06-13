//! RapidGate 程序入口（spec §5.6）

use std::process::ExitCode;
use std::sync::Arc;

use rapidgate::core::routing::{RouteTable, Router};
use rapidgate::service::config_loader;
use rapidgate::service::state::AppState;
use rapidgate::service::{self};

#[tokio::main]
async fn main() -> ExitCode {
    // 1) 加载 .env（不强制存在）
    let _ = dotenvy::dotenv();

    // 2) 初始化 tracing
    service::telemetry::init();

    // 3) 解析配置路径
    let paths = match config_loader::resolve_paths() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "resolve_paths failed");
            return ExitCode::from(78);
        }
    };

    // 4) 加载配置
    let cfg = match config_loader::load().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "config load failed");
            return ExitCode::from(1);
        }
    };

    // 5) 编译路由表
    let table = match RouteTable::new(cfg.routes.clone()) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "route table compile failed");
            return ExitCode::from(78);
        }
    };
    let router = Router::new(table);

    // 6) 构造 AppState
    let state = Arc::new(AppState::new(
        router,
        cfg.gateway.upstreams.clone(),
        cfg.gateway.defaults.rate_limit.clone(),
        paths.config_dir,
        cfg.gateway.max_body_bytes,
        cfg.gateway.request_timeout_ms,
    ));

    // 7) 启动 HTTP 服务 + graceful shutdown
    let listen = cfg.gateway.listen.clone();
    tracing::info!(listen = %listen, "rapidgate starting");

    let app = service::server::router(state);
    let listener = match tokio::net::TcpListener::bind(&listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "bind failed");
            return ExitCode::from(1);
        }
    };

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received, draining...");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        tracing::error!(error = %e, "server error");
        return ExitCode::from(1);
    }

    tracing::info!("rapidgate stopped cleanly");
    ExitCode::SUCCESS
}
