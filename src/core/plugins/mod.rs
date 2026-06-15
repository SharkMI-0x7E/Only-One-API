//! 插件系统
//!
//! 阶段三新增（spec §2 [S3]）。

pub mod native;
pub mod registry;
pub mod r#trait;
pub mod wasm;

pub use registry::PluginRegistry;
pub use r#trait::{
    ErrorContext, Plugin, PluginError, PluginMetadata, ProxyContext, RequestContext,
};
