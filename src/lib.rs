//! RapidGate — 高性能统一 LLM API 网关
//!
//! 本 crate 是单 crate 实现，`core` 模块封装无 I/O 的业务核心，
//! `service` 模块负责 axum/tokio 集成与配置加载。
//!
//! 阶段一：基础落地（spec §2 中所有 [S1] / [S1+] 标注）

pub mod core;
pub mod service;

// 重新导出核心公共 API，供 CLI / 插件复用
pub use crate::core::config::{
    AuthConfig, GatewayConfig, LoadBalancer, LoggingConfig, MatchRule, ProviderConfig,
    ProviderKind, RateLimitConfig, RouteConfig, UpstreamAllowlist, UpstreamConfig, UpstreamId,
    UpstreamPoolConfig,
};
pub use crate::core::error::CoreError;
pub use crate::core::routing::{RouteTable, Router};
pub use crate::service::config_loader::{ConfigPaths, LoadedConfig};
pub use crate::service::error::ServiceError;
pub use crate::service::state::AppState;
