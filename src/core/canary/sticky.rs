//! 会话黏性
//!
//! 基于 Cookie 的会话黏性实现，确保同一用户的请求路由到相同的 upstream。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 会话黏性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickySession {
    /// Cookie 名称
    pub cookie_name: String,
    /// 会话超时时间（秒）
    pub ttl_seconds: u64,
    /// 哈希盐值（用于生成稳定的 upstream 选择）
    pub salt: String,
}

impl StickySession {
    /// 创建新的会话黏性配置
    pub fn new(cookie_name: String, ttl_seconds: u64) -> Self {
        Self {
            cookie_name,
            ttl_seconds,
            salt: "rapidgate-sticky".to_string(),
        }
    }

    /// 从请求 Cookie 中提取会话 ID
    pub fn extract_session_id(&self, cookies: &HashMap<String, String>) -> Option<String> {
        cookies.get(&self.cookie_name).cloned()
    }

    /// 生成新的会话 ID
    pub fn generate_session_id(&self, client_ip: &str) -> String {
        use sha2::{Digest, Sha256};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        let input = format!(
            "{}:{}:{}:{}",
            self.salt,
            client_ip,
            timestamp,
            rand::random::<u32>()
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// 根据会话 ID 选择 upstream 索引
    pub fn select_upstream(&self, session_id: &str, upstream_count: usize) -> usize {
        if upstream_count == 0 {
            return 0;
        }
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(session_id.as_bytes());
        let hash = hasher.finalize();
        let hash_value = u64::from_be_bytes(hash[0..8].try_into().unwrap());
        (hash_value % upstream_count as u64) as usize
    }

    /// 检查会话是否过期
    pub fn is_expired(&self, created_at: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        now > created_at + self.ttl_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sticky_session_creation() {
        let sticky = StickySession::new("session_id".to_string(), 3600);
        assert_eq!(sticky.cookie_name, "session_id");
        assert_eq!(sticky.ttl_seconds, 3600);
    }

    #[test]
    fn test_extract_session_id() {
        let sticky = StickySession::new("session_id".to_string(), 3600);
        let mut cookies = HashMap::new();
        cookies.insert("session_id".to_string(), "abc123".to_string());

        let session_id = sticky.extract_session_id(&cookies);
        assert_eq!(session_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_select_upstream_consistency() {
        let sticky = StickySession::new("session_id".to_string(), 3600);
        let session_id = "test_session_123";

        // 同一 session_id 应该总是选择相同的 upstream
        let idx1 = sticky.select_upstream(session_id, 3);
        let idx2 = sticky.select_upstream(session_id, 3);
        let idx3 = sticky.select_upstream(session_id, 3);

        assert_eq!(idx1, idx2);
        assert_eq!(idx2, idx3);
        assert!(idx1 < 3);
    }

    #[test]
    fn test_generate_session_id() {
        let sticky = StickySession::new("session_id".to_string(), 3600);
        let session1 = sticky.generate_session_id("192.168.1.1");
        let session2 = sticky.generate_session_id("192.168.1.1");

        // 由于包含时间戳和随机数，两次生成的 session_id 应该不同
        assert_ne!(session1, session2);
        assert_eq!(session1.len(), 16);
    }
}
