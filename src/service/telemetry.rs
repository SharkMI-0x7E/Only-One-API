//! service/telemetry — tracing 初始化（spec §5）

use std::sync::OnceLock;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static INIT: OnceLock<()> = OnceLock::new();

/// 初始化全局 tracing subscriber；幂等
pub fn init() {
    INIT.get_or_init(|| {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,rapidgate=debug"));

        // 阶段一：仅 pretty；JSON 格式由 RGD_LOG_FORMAT 控制
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        init();
        init();
    }
}
