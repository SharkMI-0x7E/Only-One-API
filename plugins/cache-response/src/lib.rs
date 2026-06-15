//! cache-response 插件
//!
//! 缓存相同请求的响应，减少上游调用。

use async_trait::async_trait;
use rapidgate::core::plugins::{Plugin, PluginError, PluginMetadata, ProxyContext, RequestContext};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// 响应缓存插件
pub struct CacheResponsePlugin {
    /// 缓存存储（key: request_hash, value: response_body）
    cache: Arc<moka::future::Cache<String, Vec<u8>>>,
}

impl CacheResponsePlugin {
    /// 创建新的缓存插件
    pub fn new(max_capacity: u64) -> Self {
        let cache = moka::future::Cache::builder()
            .max_capacity(max_capacity)
            .build();
        Self {
            cache: Arc::new(cache),
        }
    }

    /// 计算请求的哈希值作为缓存 key
    fn compute_request_hash(ctx: &RequestContext) -> String {
        let mut hasher = Sha256::new();
        hasher.update(ctx.method.as_bytes());
        hasher.update(ctx.path.as_bytes());
        if let Some(body) = &ctx.body {
            hasher.update(body);
        }
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl Plugin for CacheResponsePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "cache-response".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: Some("RapidGate Team".to_string()),
            description: Some("Cache responses for identical requests".to_string()),
        }
    }

    async fn before_proxy(&self, ctx: &mut ProxyContext) -> Result<(), PluginError> {
        // 在代理前检查缓存（简化实现）
        // 实际实现需要访问 RequestContext 来计算 hash
        Ok(())
    }

    async fn after_proxy(&self, ctx: &mut ProxyContext) -> Result<(), PluginError> {
        // 在代理后存储响应到缓存（简化实现）
        Ok(())
    }
}

/// 插件入口函数（供动态库加载）
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(CacheResponsePlugin::new(1000)))
}
