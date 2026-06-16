//! 插件注册表
//!
//! 阶段三新增（spec §2 [S3]）。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::r#trait::{Plugin, PluginError, PluginMetadata};

/// 插件注册表
///
/// 管理所有已加载的插件，支持动态注册和查询。
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn Plugin>>>>,
}

impl PluginRegistry {
    /// 创建空的插件注册表
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册插件
    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError> {
        let metadata = plugin.metadata();
        let mut plugins = self.plugins.write().await;

        if plugins.contains_key(&metadata.name) {
            return Err(PluginError::InitFailed(format!(
                "plugin '{}' already registered",
                metadata.name
            )));
        }

        plugins.insert(metadata.name.clone(), plugin);
        Ok(())
    }

    /// 注销插件
    pub async fn unregister(&self, name: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        plugins
            .remove(name)
            .ok_or_else(|| PluginError::NotFound(format!("plugin '{}' not found", name)))?;
        Ok(())
    }

    /// 获取插件
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        plugins.get(name).cloned()
    }

    /// 列出所有已注册的插件
    pub async fn list(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata()).collect()
    }

    /// 获取已注册插件数量
    pub async fn count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
