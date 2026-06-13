//! service/upstream_pool — reqwest::Client 池 + SSRF 白名单基础版（spec §5 + §8）
//!
//! 阶段一实现：
//! - reqwest::Client 池（moka 缓存）
//! - 简单白名单：base_url host 必须在 `upstream_allowlist` 内
//!
//! 阶段二 [S2+] 增强：DNS 解析 + IP 段检查（spec §8）

use std::sync::Arc;
use std::time::Duration;

use crate::core::config::upstream::UpstreamConfig;
use crate::core::error::CoreError;
use crate::service::state::UpstreamCache;

pub struct UpstreamPool {
    cache: UpstreamCache,
    allowlist: Arc<Vec<String>>,
    request_timeout: Duration,
    max_body_bytes: usize,
}

impl UpstreamPool {
    pub fn new(allowlist: Vec<String>, request_timeout_ms: u64, max_body_bytes: usize) -> Self {
        Self {
            cache: UpstreamCache::builder().max_capacity(1024).build(),
            allowlist: Arc::new(allowlist),
            request_timeout: Duration::from_millis(request_timeout_ms),
            max_body_bytes,
        }
    }

    /// 检查 base_url 是否在 allowlist
    pub fn check_allowlist(&self, base_url: &str) -> Result<(), CoreError> {
        let host = base_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.split(':').next())
            .ok_or_else(|| CoreError::BadRequest(format!("invalid base_url: {base_url}")))?;
        if self.allowlist.iter().any(|h| h.eq_ignore_ascii_case(host)) {
            Ok(())
        } else {
            Err(CoreError::BadRequest(format!(
                "upstream host '{host}' not in allowlist"
            )))
        }
    }

    /// 拿到（或构造并缓存）该 upstream 对应的 reqwest::Client
    pub async fn client_for(&self, up: &UpstreamConfig) -> Result<Arc<reqwest::Client>, CoreError> {
        self.check_allowlist(&up.base_url)?;
        if let Some(c) = self.cache.get(&up.id).await {
            return Ok(c);
        }
        let client = reqwest::Client::builder()
            .timeout(self.request_timeout)
            .user_agent(concat!("rapidgate/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::UpstreamUnreachable(format!("client build: {e}")))?;
        let arc = Arc::new(client);
        self.cache.insert(up.id.clone(), arc.clone()).await;
        Ok(arc)
    }

    pub fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
    }
}
