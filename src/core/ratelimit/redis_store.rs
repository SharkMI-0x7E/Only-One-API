//! Redis 分布式限流存储（spec §4.5）
//!
//! 使用 Redis 作为后端存储，支持令牌桶和滑动窗口算法。
//! 所有操作使用 Lua 脚本保证原子性。

use async_trait::async_trait;

use super::{Decision, LimitKey, RateLimiter};
use crate::core::error::CoreError;

/// Redis 限流存储
pub struct RedisStore {
    client: redis::Client,
}

impl RedisStore {
    /// 创建 Redis 存储
    pub fn new(redis_url: &str) -> Result<Self, CoreError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| CoreError::Internal(format!("failed to create redis client: {e}")))?;
        Ok(Self { client })
    }

    /// 获取异步连接
    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, CoreError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CoreError::Internal(format!("failed to get redis connection: {e}")))
    }
}

/// 令牌桶 Lua 脚本
const TOKEN_BUCKET_SCRIPT: &str = r#"
local key = KEYS[1]
local capacity = tonumber(ARGV[1])
local rate = tonumber(ARGV[2])
local now = tonumber(ARGV[3])
local requested = tonumber(ARGV[4])

local bucket = redis.call('HMGET', key, 'tokens', 'last_time')
local tokens = tonumber(bucket[1])
local last_time = tonumber(bucket[2])

if tokens == nil then
    tokens = capacity
    last_time = now
end

local elapsed = math.max(0, now - last_time)
tokens = math.min(capacity, tokens + elapsed * rate)

if tokens >= requested then
    tokens = tokens - requested
    redis.call('HMSET', key, 'tokens', tokens, 'last_time', now)
    redis.call('EXPIRE', key, math.ceil(capacity / rate) * 2)
    return 1
else
    return 0
end
"#;

/// 滑动窗口 Lua 脚本
const SLIDING_WINDOW_SCRIPT: &str = r#"
local key = KEYS[1]
local limit = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local now = tonumber(ARGV[3])

local window_start = now - window

redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)
local count = redis.call('ZCARD', key)

if count < limit then
    redis.call('ZADD', key, now, now .. '-' .. math.random(1000000))
    redis.call('EXPIRE', key, window * 2)
    return 1
else
    return 0
end
"#;

/// 令牌桶配置
#[derive(Debug, Clone)]
pub struct TokenBucketConfig {
    /// 桶容量
    pub capacity: u32,
    /// 补充速率（tokens/sec）
    pub rate: f64,
}

/// 滑动窗口配置
#[derive(Debug, Clone)]
pub struct SlidingWindowConfig {
    /// 窗口大小（秒）
    pub window: u64,
    /// 窗口内最大请求数
    pub limit: u32,
}

/// 限流算法
pub enum Algorithm {
    TokenBucket(TokenBucketConfig),
    SlidingWindow(SlidingWindowConfig),
}

/// Redis 限流器
pub struct RedisRateLimiter {
    store: RedisStore,
    algorithm: Algorithm,
}

impl RedisRateLimiter {
    /// 创建令牌桶限流器
    pub fn token_bucket(redis_url: &str, config: TokenBucketConfig) -> Result<Self, CoreError> {
        let store = RedisStore::new(redis_url)?;
        Ok(Self {
            store,
            algorithm: Algorithm::TokenBucket(config),
        })
    }

    /// 创建滑动窗口限流器
    pub fn sliding_window(redis_url: &str, config: SlidingWindowConfig) -> Result<Self, CoreError> {
        let store = RedisStore::new(redis_url)?;
        Ok(Self {
            store,
            algorithm: Algorithm::SlidingWindow(config),
        })
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, key: &LimitKey) -> Result<Decision, CoreError> {
        let mut conn = self.store.get_connection().await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| CoreError::Internal(format!("failed to get timestamp: {e}")))?
            .as_secs_f64();

        match &self.algorithm {
            Algorithm::TokenBucket(config) => {
                let script = redis::Script::new(TOKEN_BUCKET_SCRIPT);
                let result: i32 = script
                    .key(key)
                    .arg(config.capacity)
                    .arg(config.rate)
                    .arg(now)
                    .arg(1) // requested tokens
                    .invoke_async(&mut conn)
                    .await
                    .map_err(|e| {
                        CoreError::Internal(format!("failed to execute token bucket script: {e}"))
                    })?;

                if result == 1 {
                    Ok(Decision::Allow)
                } else {
                    Ok(Decision::Deny)
                }
            }
            Algorithm::SlidingWindow(config) => {
                let script = redis::Script::new(SLIDING_WINDOW_SCRIPT);
                let result: i32 = script
                    .key(key)
                    .arg(config.limit)
                    .arg(config.window)
                    .arg(now)
                    .invoke_async(&mut conn)
                    .await
                    .map_err(|e| {
                        CoreError::Internal(format!("failed to execute sliding window script: {e}"))
                    })?;

                if result == 1 {
                    Ok(Decision::Allow)
                } else {
                    Ok(Decision::Deny)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_config() {
        let config = TokenBucketConfig {
            capacity: 100,
            rate: 10.0,
        };
        assert_eq!(config.capacity, 100);
        assert_eq!(config.rate, 10.0);
    }

    #[test]
    fn test_sliding_window_config() {
        let config = SlidingWindowConfig {
            window: 60,
            limit: 1000,
        };
        assert_eq!(config.window, 60);
        assert_eq!(config.limit, 1000);
    }
}
