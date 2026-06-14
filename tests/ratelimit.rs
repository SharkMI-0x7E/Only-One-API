//! 限流集成测试（spec §5.3）

use rapidgate::core::ratelimit::sliding_window::SlidingWindow;
use rapidgate::core::ratelimit::token_bucket::TokenBucket;
use rapidgate::core::ratelimit::{Decision, RateLimiter};

#[tokio::test]
async fn token_bucket_allows_burst() {
    let limiter = TokenBucket::new(1, 5);
    for _ in 0..5 {
        assert_eq!(
            limiter.check(&"test-key".into()).await.unwrap(),
            Decision::Allow
        );
    }
}

#[tokio::test]
async fn token_bucket_denies_over_burst() {
    let limiter = TokenBucket::new(1, 2);
    assert_eq!(
        limiter.check(&"test-key".into()).await.unwrap(),
        Decision::Allow
    );
    assert_eq!(
        limiter.check(&"test-key".into()).await.unwrap(),
        Decision::Allow
    );
    assert_eq!(
        limiter.check(&"test-key".into()).await.unwrap(),
        Decision::Deny
    );
}

#[tokio::test]
async fn sliding_window_allows_within_limit() {
    let limiter = SlidingWindow::new(5, 1000);
    for _ in 0..5 {
        assert_eq!(
            limiter.check(&"test-key".into()).await.unwrap(),
            Decision::Allow
        );
    }
}

#[tokio::test]
async fn sliding_window_denies_over_limit() {
    let limiter = SlidingWindow::new(2, 1000);
    assert_eq!(
        limiter.check(&"test-key".into()).await.unwrap(),
        Decision::Allow
    );
    assert_eq!(
        limiter.check(&"test-key".into()).await.unwrap(),
        Decision::Allow
    );
    assert_eq!(
        limiter.check(&"test-key".into()).await.unwrap(),
        Decision::Deny
    );
}
