//! 熔断器实现（spec §4.5）
//!
//! 失败次数达到阈值 → Open；Open 持续到 `open_duration_ms` 后 → HalfOpen；
//! HalfOpen 时一次成功 → Closed，一次失败 → Open。

use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::core::breaker::state::BreakerState;
use crate::core::error::CoreError;

pub struct Breaker {
    name: String,
    failure_threshold: u32,
    open_duration: Duration,
    state: Mutex<BreakerState>,
    consecutive_failures: Mutex<u32>,
    opened_at: Mutex<Option<Instant>>,
}

impl Breaker {
    pub fn new(name: impl Into<String>, failure_threshold: u32, open_duration_ms: u64) -> Self {
        Self {
            name: name.into(),
            failure_threshold,
            open_duration: Duration::from_millis(open_duration_ms),
            state: Mutex::new(BreakerState::Closed),
            consecutive_failures: Mutex::new(0),
            opened_at: Mutex::new(None),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state(&self) -> BreakerState {
        *self.state.lock().expect("breaker state lock poisoned")
    }

    fn on_success(&self) {
        let mut state = self.state.lock().expect("breaker state lock poisoned");
        let mut fail = self
            .consecutive_failures
            .lock()
            .expect("breaker fail lock poisoned");
        let mut opened = self.opened_at.lock().expect("breaker opened lock poisoned");
        *state = BreakerState::Closed;
        *fail = 0;
        *opened = None;
    }

    fn on_failure(&self) {
        let mut state = self.state.lock().expect("breaker state lock poisoned");
        let mut fail = self
            .consecutive_failures
            .lock()
            .expect("breaker fail lock poisoned");
        *fail += 1;
        if *fail >= self.failure_threshold {
            *state = BreakerState::Open;
            *self.opened_at.lock().expect("breaker opened lock poisoned") = Some(Instant::now());
        }
    }

    fn try_half_open(&self) {
        let mut state = self.state.lock().expect("breaker state lock poisoned");
        if *state == BreakerState::Open {
            let opened_at = *self.opened_at.lock().expect("breaker opened lock poisoned");
            if let Some(t) = opened_at {
                if t.elapsed() >= self.open_duration {
                    *state = BreakerState::HalfOpen;
                }
            }
        }
    }

    /// 用熔断器包裹一个 future；Open 状态直接拒绝
    pub async fn call<F, T>(&self, fut: F) -> Result<T, CoreError>
    where
        F: Future<Output = Result<T, CoreError>>,
    {
        self.try_half_open();
        let s = self.state();
        match s {
            BreakerState::Open => return Err(CoreError::BreakerOpen(self.name.clone())),
            BreakerState::Closed | BreakerState::HalfOpen => {}
        }

        match fut.await {
            Ok(v) => {
                self.on_success();
                Ok(v)
            }
            Err(e) => {
                self.on_failure();
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn opens_after_threshold() {
        let b = Breaker::new("test", 2, 1000);
        for _ in 0..2 {
            let _ = b
                .call(async { Err::<(), _>(CoreError::Internal("x".into())) })
                .await;
        }
        assert_eq!(b.state(), BreakerState::Open);
        let r = b.call(async { Ok::<_, CoreError>(()) }).await;
        assert!(matches!(r, Err(CoreError::BreakerOpen(_))));
    }

    #[tokio::test]
    async fn closes_on_success() {
        let b = Breaker::new("test", 1, 10);
        let _ = b
            .call(async { Err::<(), _>(CoreError::Internal("x".into())) })
            .await;
        assert_eq!(b.state(), BreakerState::Open);
        // 等待 open_duration 过去
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let r = b.call(async { Ok::<_, CoreError>(()) }).await;
        assert!(r.is_ok());
        assert_eq!(b.state(), BreakerState::Closed);
    }
}
