//! GatewayConfig — 网关总配置（spec §6.1）

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConfig {
    pub listen: String,
    pub request_timeout_ms: u64,
    pub max_body_bytes: usize,
    pub shutdown_timeout_ms: u64,
    pub logging: LoggingConfig,
    pub upstream_allowlist: UpstreamAllowlist,
    pub defaults: Defaults,
    pub upstreams: Vec<crate::core::config::upstream::UpstreamConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamAllowlist {
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    pub rate_limit: crate::core::config::route::RateLimitConfig,
    pub breaker: BreakerDefaults,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreakerDefaults {
    pub failure_threshold: u32,
    pub open_duration_ms: u64,
}
