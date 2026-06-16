//! core/ratelimit — 限流算法（spec §4.5）
//!
//! 阶段一实现：令牌桶 + 滑动窗口 + Moka 进程内存储
//! 阶段三新增：Redis 分布式限流存储

pub mod local_store;
pub mod redis_store;
pub mod sliding_window;
pub mod token_bucket;

use async_trait::async_trait;

use crate::core::error::CoreError;

/// 限流 key（如 user_id / api_key fingerprint / IP）
pub type LimitKey = String;

/// 限流结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// 通用限流器 trait（不绑死 axum / tokio 句柄）
#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check(&self, key: &LimitKey) -> Result<Decision, CoreError>;
}
