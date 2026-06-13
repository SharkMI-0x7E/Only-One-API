//! RouteConfig + RouteMatch（spec §4.2 / §6.2）

use serde::Deserialize;

use crate::core::config::upstream::UpstreamId;

/// 单条路由配置
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteConfig {
    pub name: String,
    #[serde(rename = "match")]
    pub match_rule: MatchRule,
    pub upstream: UpstreamRef,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

/// 路由匹配条件（spec §4.3）
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchRule {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub headers: Vec<HeaderMatch>,
    #[serde(default)]
    pub query: Vec<QueryMatch>,
}

/// Header 匹配
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderMatch {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub regex: bool,
}

/// Query 参数匹配
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryMatch {
    pub name: String,
    pub value: String,
}

/// 路由 → 上游引用
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamRef {
    pub id: UpstreamId,
}

/// 鉴权配置（阶段一仅支持 bearer / apikey）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    #[serde(rename = "type", default)]
    pub kind: AuthKind,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    #[default]
    None,
    Bearer,
    ApiKey,
}

/// 速率限制（默认从 `defaults.rate_limit` 继承）
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitConfig {
    pub algorithm: String,
    pub rps: u32,
    pub burst: u32,
}
