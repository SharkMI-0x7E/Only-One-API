//! core/config — 配置数据模型（仅 serde 定义，不负责加载）

pub mod gateway;
pub mod route;
pub mod upstream;

pub use gateway::{GatewayConfig, LoggingConfig, UpstreamAllowlist};
pub use route::{AuthConfig, HeaderMatch, MatchRule, QueryMatch, RateLimitConfig, RouteConfig};
pub use upstream::{LoadBalancer, UpstreamConfig, UpstreamId};
