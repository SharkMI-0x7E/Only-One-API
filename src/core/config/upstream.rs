//! UpstreamConfig + LoadBalancer（spec §4.2）

use serde::Deserialize;

/// 上游 ID 字符串（kebab-case，spec §9.4）
pub type UpstreamId = String;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamConfig {
    pub id: UpstreamId,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub load_balancer: LoadBalancer,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancer {
    #[default]
    RoundRobin,
    Random,
    // 一致性哈希留 [S2]
}
