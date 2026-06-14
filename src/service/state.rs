//! AppState — 跨请求共享状态（spec §5.2）
//!
//! 全部 Arc 共享，通过 `axum::extract::State` 注入 handler。

use std::path::PathBuf;
use std::sync::Arc;

use moka::future::Cache;
use tokio::sync::mpsc;

use crate::core::audit::AuditEvent;
use crate::core::config::route::RateLimitConfig;
use crate::core::config::upstream::{UpstreamConfig, UpstreamId};
use crate::core::routing::Router;

pub type UpstreamCache = Cache<UpstreamId, Arc<reqwest::Client>>;
pub type LimiterCache = Cache<String, Arc<dyn crate::core::ratelimit::RateLimiter>>;

pub struct AppState {
    pub route_table: Router,
    pub upstreams: UpstreamCache,
    pub limiters: LimiterCache,
    pub audit_tx: mpsc::UnboundedSender<AuditEvent>,
    pub config_dir: PathBuf,
    pub max_body_bytes: usize,
    pub request_timeout_ms: u64,
    pub upstream_configs: Vec<UpstreamConfig>,
    pub default_rate_limit: RateLimitConfig,
}

impl AppState {
    pub fn new(
        route_table: Router,
        upstream_configs: Vec<UpstreamConfig>,
        default_rate_limit: RateLimitConfig,
        config_dir: PathBuf,
        max_body_bytes: usize,
        request_timeout_ms: u64,
    ) -> Self {
        let upstreams: UpstreamCache = Cache::builder().max_capacity(1024).build();
        let limiters: LimiterCache = Cache::builder().max_capacity(1024).build();
        let (audit_tx, _audit_rx) = mpsc::unbounded_channel();
        Self {
            route_table,
            upstreams,
            limiters,
            audit_tx,
            config_dir,
            max_body_bytes,
            request_timeout_ms,
            upstream_configs,
            default_rate_limit,
        }
    }
}
