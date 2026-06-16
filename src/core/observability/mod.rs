//! core/observability — 可观测性抽象（spec §4.6）
//!
//! 阶段一：TraceId
//! 阶段三：Prometheus 指标 + OpenTelemetry 导出

pub mod metrics;
pub mod otel;
pub mod trace;

pub use metrics::Metrics;
pub use otel::{OtelConfig, OtelError, OtelInitializer};
pub use trace::{TraceContext, TraceId};
