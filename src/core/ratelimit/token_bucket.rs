//! 令牌桶限流（spec §4.5）

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;

use crate::core::error::CoreError;
use crate::core::ratelimit::{Decision, LimitKey, RateLimiter};

/// 令牌桶状态
#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

pub struct TokenBucket {
    rps: f64,
    burst: f64,
    buckets: Mutex<HashMap<LimitKey, Bucket>>,
}

impl TokenBucket {
    pub fn new(rps: u32, burst: u32) -> Self {
        Self {
            rps: rps as f64,
            burst: burst as f64,
            buckets: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl RateLimiter for TokenBucket {
    async fn check(&self, key: &LimitKey) -> Result<Decision, CoreError> {
        let mut buckets = self.buckets.lock().expect("token bucket lock poisoned");
        let now = Instant::now();
        let entry = buckets.entry(key.clone()).or_insert(Bucket {
            tokens: self.burst,
            last_refill: now,
        });
        // 补充令牌
        let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
        entry.tokens = (entry.tokens + elapsed * self.rps).min(self.burst);
        entry.last_refill = now;

        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            Ok(Decision::Allow)
        } else {
            Ok(Decision::Deny)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_burst() {
        let lim = TokenBucket::new(1, 5);
        for _ in 0..5 {
            assert_eq!(lim.check(&"u1".into()).await.unwrap(), Decision::Allow);
        }
    }

    #[tokio::test]
    async fn denies_over_burst() {
        let lim = TokenBucket::new(1, 2);
        assert_eq!(lim.check(&"u1".into()).await.unwrap(), Decision::Allow);
        assert_eq!(lim.check(&"u1".into()).await.unwrap(), Decision::Allow);
        assert_eq!(lim.check(&"u1".into()).await.unwrap(), Decision::Deny);
    }

    #[tokio::test]
    async fn per_key_isolation() {
        let lim = TokenBucket::new(1, 1);
        assert_eq!(lim.check(&"a".into()).await.unwrap(), Decision::Allow);
        assert_eq!(lim.check(&"a".into()).await.unwrap(), Decision::Deny);
        assert_eq!(lim.check(&"b".into()).await.unwrap(), Decision::Allow);
    }
}
