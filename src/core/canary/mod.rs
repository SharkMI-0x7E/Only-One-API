//! 灰度发布模块
//!
//! 支持按权重、Header、Cookie 将流量分发到不同 upstream。

pub mod policy;
pub mod sticky;

pub use policy::CanaryPolicy;
pub use sticky::StickySession;
