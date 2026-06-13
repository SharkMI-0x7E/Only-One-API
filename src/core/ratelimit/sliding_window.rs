//! 滑动窗口限流（spec §4.5）

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::core::error::CoreError;
use crate::core::ratelimit::{Decision, LimitKey, RateLimiter};

pub struct SlidingWindow {
    window: Duration,
    max_requests: usize,
    buckets: Mutex<HashMap<LimitKey, VecDeque<Instant>>>,
}

impl SlidingWindow {
    pub fn new(rps: u32, _burst: u32) -> Self {
        // rps 决定窗口长度，窗口内允许 rps 次
        Self {
            window: Duration::from_secs(1)
                .checked_div(rps.max(1))
                .unwrap_or(Duration::from_secs(1)),
            max_requests: rps as usize,
            buckets: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl RateLimiter for SlidingWindow {
    async fn check(&self, key: &LimitKey) -> Result<Decision, CoreError> {
        let mut buckets = self.buckets.lock().expect("sliding window lock poisoned");
        let now = Instant::now();
        let entry = buckets.entry(key.clone()).or_default();

        // 弹出窗口外的旧记录
        while let Some(&front) = entry.front() {
            if now.duration_since(front) > self.window {
                entry.pop_front();
            } else {
                break;
            }
        }

        if entry.len() < self.max_requests {
            entry.push_back(now);
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
    async fn allows_up_to_rps() {
        let lim = SlidingWindow::new(3, 3);
        for _ in 0..3 {
            assert_eq!(lim.check(&"u1".into()).await.unwrap(), Decision::Allow);
        }
        assert_eq!(lim.check(&"u1".into()).await.unwrap(), Decision::Deny);
    }
}
