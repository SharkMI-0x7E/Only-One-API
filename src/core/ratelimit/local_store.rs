//! 进程内 Moka 存储（spec §4.5）
//!
//! 阶段一占位：用于阶段二 [S2+] 缓存 token 桶/滑动窗口实例
//! 以及阶段三 [S3] OAuth2 token 缓存

use std::time::Duration;

use moka::future::Cache;

/// 通用本地缓存封装
pub struct LocalStore<K, V>
where
    K: std::hash::Hash + Eq + Send + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    inner: Cache<K, V>,
}

impl<K, V> LocalStore<K, V>
where
    K: std::hash::Hash + Eq + Send + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let inner = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();
        Self { inner }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).await
    }

    pub async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value).await;
    }

    pub async fn invalidate(&self, key: &K) {
        self.inner.invalidate(key).await;
    }
}
