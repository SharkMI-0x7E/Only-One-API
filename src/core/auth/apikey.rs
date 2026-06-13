//! API Key 认证（spec §4.5）
//!
//! **关键约束**：用 `subtle::ConstantTimeEq` 常量时间比较，**禁止**用 `==` / `String::eq`
//! 直接比较 API Key。错误响应**不**区分"key 不存在" vs "key 错误"，统一返回
//! `CoreError::Auth`，由 `ServiceError` 映射为 `unauthorized`。

use std::collections::HashSet;
use std::sync::RwLock;

use async_trait::async_trait;
use subtle::ConstantTimeEq;

use crate::core::auth::Authenticator;
use crate::core::error::CoreError;

/// API Key 认证器：维护一组合法 key
pub struct ApiKeyAuthenticator {
    keys: RwLock<HashSet<String>>,
}

impl ApiKeyAuthenticator {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashSet::new()),
        }
    }

    /// 注册一个新 key（启动时从环境变量加载）
    pub fn register(&self, key: String) {
        let mut guard = self.keys.write().expect("api key lock poisoned");
        guard.insert(key);
    }

    /// 注销（热重载时使用，留 [S2]）
    #[allow(dead_code)]
    pub fn unregister(&self, key: &str) {
        let mut guard = self.keys.write().expect("api key lock poisoned");
        guard.remove(key);
    }

    pub fn is_empty(&self) -> bool {
        self.keys.read().expect("api key lock poisoned").is_empty()
    }
}

impl Default for ApiKeyAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Authenticator for ApiKeyAuthenticator {
    async fn verify(&self, credential: &str) -> Result<(), CoreError> {
        let guard = self.keys.read().expect("api key lock poisoned");

        // 常量时间比较：O(n) × 全部 key，但每个 key 比较本身是常量时间
        // 不论命中与否、不论 key 集合大小，耗时差异极小
        let mut ok: u8 = 0;
        for stored in guard.iter() {
            // 长度不同则直接跳过该 key（不影响其他 key 的常量时间）
            if stored.len() != credential.len() {
                continue;
            }
            let a = stored.as_bytes();
            let b = credential.as_bytes();
            // 长度相同才走 ConstantTimeEq
            if a.ct_eq(b).into() {
                ok = 1;
            }
        }

        if ok == 1 {
            Ok(())
        } else {
            // 统一错误：**不**区分"key 不存在" vs "key 错误"
            Err(CoreError::Auth("invalid or missing api key".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepts_registered_key() {
        let auth = ApiKeyAuthenticator::new();
        auth.register("sk-test-1234567890".to_string());
        assert!(auth.verify("sk-test-1234567890").await.is_ok());
    }

    #[tokio::test]
    async fn rejects_unknown_key() {
        let auth = ApiKeyAuthenticator::new();
        auth.register("sk-test-1234567890".to_string());
        assert!(auth.verify("sk-other-0987654321").await.is_err());
    }

    #[tokio::test]
    async fn rejects_empty_credential() {
        let auth = ApiKeyAuthenticator::new();
        auth.register("sk-test-1234567890".to_string());
        assert!(auth.verify("").await.is_err());
    }

    #[tokio::test]
    async fn uniform_error_message() {
        let auth = ApiKeyAuthenticator::new();
        auth.register("sk-test-1234567890".to_string());

        let a = auth.verify("sk-wrong").await.unwrap_err();
        let b = auth.verify("").await.unwrap_err();
        // 错误消息必须**完全相同**，避免侧信道
        assert_eq!(a.to_string(), b.to_string());
    }
}
