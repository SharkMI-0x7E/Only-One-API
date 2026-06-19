//! service 模块 — axum + tokio + 文件 I/O 集成层

pub mod admin;
pub mod config_center;
pub mod config_loader;
pub mod error;
pub mod handler;
pub mod hot_reload;
pub mod middleware;
pub mod providers;
pub mod server;
pub mod state;
pub mod telemetry;
pub mod upstream_pool;

pub use error::ServiceError;
pub use state::AppState;
