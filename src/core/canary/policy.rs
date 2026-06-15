//! 灰度策略
//!
//! 定义权重、Header、Cookie 三种灰度策略。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 灰度策略类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanaryPolicy {
    /// 按权重分配流量
    Weight {
        /// 主版本权重（0-100）
        stable_weight: u32,
        /// 灰度版本权重（0-100）
        canary_weight: u32,
    },
    /// 按 Header 匹配
    Header {
        /// Header 名称
        header_name: String,
        /// Header 值（匹配则路由到灰度版本）
        header_value: String,
    },
    /// 按 Cookie 黏性会话
    Cookie {
        /// Cookie 名称
        cookie_name: String,
        /// 会话超时时间（秒）
        ttl_seconds: u64,
    },
}

impl CanaryPolicy {
    /// 根据请求上下文判断是否应该路由到灰度版本
    pub fn should_route_to_canary(
        &self,
        headers: &HashMap<String, String>,
        cookies: &HashMap<String, String>,
    ) -> bool {
        match self {
            CanaryPolicy::Weight {
                stable_weight,
                canary_weight,
            } => {
                // 简单实现：使用随机数
                let total = stable_weight + canary_weight;
                if total == 0 {
                    return false;
                }
                let random = rand::random::<u32>() % total;
                random < *canary_weight
            }
            CanaryPolicy::Header {
                header_name,
                header_value,
            } => headers
                .get(header_name)
                .map(|v| v == header_value)
                .unwrap_or(false),
            CanaryPolicy::Cookie { cookie_name, .. } => cookies.contains_key(cookie_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_policy() {
        let policy = CanaryPolicy::Weight {
            stable_weight: 80,
            canary_weight: 20,
        };
        let headers = HashMap::new();
        let cookies = HashMap::new();
        // 权重策略是随机的，这里只测试不 panic
        let _ = policy.should_route_to_canary(&headers, &cookies);
    }

    #[test]
    fn test_header_policy() {
        let policy = CanaryPolicy::Header {
            header_name: "x-canary".to_string(),
            header_value: "true".to_string(),
        };
        let mut headers = HashMap::new();
        headers.insert("x-canary".to_string(), "true".to_string());
        let cookies = HashMap::new();
        assert!(policy.should_route_to_canary(&headers, &cookies));
    }

    #[test]
    fn test_cookie_policy() {
        let policy = CanaryPolicy::Cookie {
            cookie_name: "session".to_string(),
            ttl_seconds: 3600,
        };
        let headers = HashMap::new();
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "abc123".to_string());
        assert!(policy.should_route_to_canary(&headers, &cookies));
    }
}
