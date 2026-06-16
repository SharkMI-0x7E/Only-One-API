use rand::Rng;
use std::collections::HashMap;

/// 灰度策略 trait
pub trait CanaryPolicy: Send + Sync {
    /// 选择 upstream 索引
    fn select_upstream(
        &self,
        headers: &HashMap<String, String>,
        cookies: &HashMap<String, String>,
        client_ip: &str,
        upstream_count: usize,
    ) -> usize;
}

/// 权重策略：按权重随机选择
#[derive(Debug, Clone)]
pub struct WeightPolicy {
    /// 主 upstream 权重（0-100）
    pub primary_weight: u8,
}

impl WeightPolicy {
    /// 创建权重策略
    pub fn new(primary_weight: u8) -> Self {
        Self {
            primary_weight: primary_weight.min(100),
        }
    }
}

impl CanaryPolicy for WeightPolicy {
    fn select_upstream(
        &self,
        _headers: &HashMap<String, String>,
        _cookies: &HashMap<String, String>,
        _client_ip: &str,
        upstream_count: usize,
    ) -> usize {
        if upstream_count == 0 {
            return 0;
        }
        if upstream_count == 1 {
            return 0;
        }

        // 按权重选择：primary_weight% 概率选第一个，其余概率选其他
        let mut rng = rand::thread_rng();
        let rand_val: u8 = rng.gen_range(0..100);

        if rand_val < self.primary_weight {
            0 // 选主 upstream
        } else {
            // 在其他 upstream 中随机选择
            rng.gen_range(1..upstream_count)
        }
    }
}

/// Header 策略：根据请求头匹配
#[derive(Debug, Clone)]
pub struct HeaderPolicy {
    /// 要匹配的 header 名称
    pub header_name: String,
    /// 要匹配的值
    pub header_value: String,
    /// 匹配时选择的 upstream 索引
    pub target_index: usize,
}

impl CanaryPolicy for HeaderPolicy {
    fn select_upstream(
        &self,
        headers: &HashMap<String, String>,
        _cookies: &HashMap<String, String>,
        _client_ip: &str,
        upstream_count: usize,
    ) -> usize {
        if let Some(value) = headers.get(&self.header_name) {
            if value == &self.header_value && self.target_index < upstream_count {
                return self.target_index;
            }
        }
        // 不匹配时返回第一个
        0
    }
}

/// Cookie 策略：根据 Cookie 匹配
#[derive(Debug, Clone)]
pub struct CookiePolicy {
    /// 要匹配的 cookie 名称
    pub cookie_name: String,
    /// 要匹配的值
    pub cookie_value: String,
    /// 匹配时选择的 upstream 索引
    pub target_index: usize,
}

impl CanaryPolicy for CookiePolicy {
    fn select_upstream(
        &self,
        _headers: &HashMap<String, String>,
        cookies: &HashMap<String, String>,
        _client_ip: &str,
        upstream_count: usize,
    ) -> usize {
        if let Some(value) = cookies.get(&self.cookie_name) {
            if value == &self.cookie_value && self.target_index < upstream_count {
                return self.target_index;
            }
        }
        // 不匹配时返回第一个
        0
    }
}
