//! add-request-id 插件
//!
//! 自动为请求注入唯一的 request-id header（如果不存在）。

use async_trait::async_trait;
use rapidgate::core::plugins::{Plugin, PluginError, PluginMetadata, RequestContext};
use uuid::Uuid;

/// 自动注入 request-id 的插件
pub struct AddRequestIdPlugin;

#[async_trait]
impl Plugin for AddRequestIdPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "add-request-id".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: Some("RapidGate Team".to_string()),
            description: Some("Auto-inject unique request-id header".to_string()),
        }
    }

    async fn on_request(&self, ctx: &mut RequestContext) -> Result<(), PluginError> {
        // 如果请求头中没有 request-id，则生成一个
        if !ctx.headers.contains_key("x-request-id") {
            let request_id = Uuid::new_v4().to_string();
            ctx.headers.insert("x-request-id".to_string(), request_id);
        }
        Ok(())
    }
}

/// 插件入口函数（供动态库加载）
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(AddRequestIdPlugin))
}
