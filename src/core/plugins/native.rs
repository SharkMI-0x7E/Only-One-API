//! Native 插件加载器
//!
//! 阶段三新增（spec §2 [S3]）。

use std::path::Path;
use std::sync::Arc;

use super::r#trait::{Plugin, PluginError};

/// Native 插件加载器
///
/// 从动态库（.so / .dll / .dylib）加载插件。
pub struct NativePluginLoader;

impl NativePluginLoader {
    /// 创建 native 插件加载器
    pub fn new() -> Self {
        Self
    }

    /// 从动态库加载插件
    ///
    /// # Safety
    ///
    /// 加载动态库并调用 `create_plugin` 函数。该函数必须存在且签名正确。
    pub fn load_from_library<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Arc<dyn Plugin>, PluginError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(PluginError::NotFound(format!(
                "plugin library not found: {}",
                path.display()
            )));
        }

        // 实际实现需要使用 libloading crate 加载动态库
        // 这里提供骨架实现
        Err(PluginError::InitFailed(format!(
            "native plugin loading not yet implemented: {}",
            path.display()
        )))
    }
}

impl Default for NativePluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
