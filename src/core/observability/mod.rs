//! core/observability — 可观测性抽象（spec §4.6）
//!
//! 阶段一只落地 `TraceId`；W3C tracecontext 完整实现与 prometheus/OTel 留 [S2] / [S3]。

pub mod trace;

pub use trace::{TraceContext, TraceId};
