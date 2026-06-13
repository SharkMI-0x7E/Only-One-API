//! service 模块 — axum + tokio + 文件 I/O 集成层

pub mod config_loader;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod server;
pub mod state;
pub mod telemetry;
pub mod upstream_pool;

pub use error::ServiceError;
pub use state::AppState;
