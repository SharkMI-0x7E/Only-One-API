//! RapidGate — 高性能统一 LLM API 网关
//!
//! 本 crate 是单 crate 实现，`core` 模块封装无 I/O 的业务核心，
//! `service` 模块负责 axum/tokio 集成与配置加载。
//!
//! 阶段一：基础落地（spec §2 中所有 [S1] / [S1+] 标注）

pub mod core;
pub mod service;
