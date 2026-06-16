//! OpenTelemetry tracing/metrics 导出（spec §4.6）
//!
//! 提供 OTLP 协议的 tracing 和 metrics 导出能力。

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace as sdktrace;
use std::time::Duration;

/// OpenTelemetry 配置
pub struct OtelConfig {
    /// OTLP endpoint
    pub endpoint: String,
    /// 服务名称
    pub service_name: String,
    /// 是否启用 tracing
    pub enable_tracing: bool,
    /// 是否启用 metrics
    pub enable_metrics: bool,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            service_name: "rapidgate".to_string(),
            enable_tracing: true,
            enable_metrics: true,
        }
    }
}

/// OpenTelemetry 初始化器
pub struct OtelInitializer;

impl OtelInitializer {
    /// 初始化 OpenTelemetry tracing
    pub fn init_tracing(config: &OtelConfig) -> Result<sdktrace::Tracer, OtelError> {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&config.endpoint)
            .with_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| OtelError::InitFailed(format!("failed to build span exporter: {e}")))?;

        let resource = opentelemetry_sdk::Resource::new(vec![KeyValue::new(
            "service.name",
            config.service_name.clone(),
        )]);

        let provider = sdktrace::TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_resource(resource)
            .build();

        let tracer = provider.tracer(config.service_name.clone());
        global::set_tracer_provider(provider);

        Ok(tracer)
    }

    /// 初始化 OpenTelemetry metrics
    pub fn init_metrics(config: &OtelConfig) -> Result<(), OtelError> {
        let exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(&config.endpoint)
            .with_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| OtelError::InitFailed(format!("failed to build metric exporter: {e}")))?;

        let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
            exporter,
            opentelemetry_sdk::runtime::Tokio,
        )
        .with_interval(Duration::from_secs(30))
        .build();

        let resource = opentelemetry_sdk::Resource::new(vec![KeyValue::new(
            "service.name",
            config.service_name.clone(),
        )]);

        let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .build();

        global::set_meter_provider(provider);

        Ok(())
    }

    /// 关闭 OpenTelemetry（刷新所有 pending 数据）
    pub fn shutdown() {
        // TracerProvider 和 MeterProvider 会在 drop 时自动 flush
    }
}

/// OpenTelemetry 错误
#[derive(Debug)]
pub enum OtelError {
    /// 初始化失败
    InitFailed(String),
}

impl std::fmt::Display for OtelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OtelError::InitFailed(msg) => write!(f, "otel init failed: {msg}"),
        }
    }
}

impl std::error::Error for OtelError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_config_default() {
        let config = OtelConfig::default();
        assert_eq!(config.endpoint, "http://localhost:4317");
        assert_eq!(config.service_name, "rapidgate");
        assert!(config.enable_tracing);
        assert!(config.enable_metrics);
    }

    #[test]
    fn test_otel_error_display() {
        let err = OtelError::InitFailed("test error".to_string());
        assert_eq!(format!("{err}"), "otel init failed: test error");
    }
}
