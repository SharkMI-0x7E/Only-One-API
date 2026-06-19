//! core 模块 — 无 I/O / 无网络 / 无 tokio 的业务核心
//!
//! 只做数据建模、trait 抽象、纯算法。所有跨进程共享状态、文件 I/O、
//! HTTP 客户端调用均在 `service` 模块实现。

pub mod audit;
pub mod auth;
pub mod breaker;
pub mod canary;
pub mod config;
pub mod error;
pub mod observability;
pub mod plugins;
pub mod provider;
pub mod proxy;
pub mod ratelimit;
pub mod routing;
pub mod util;

pub use error::CoreError;
