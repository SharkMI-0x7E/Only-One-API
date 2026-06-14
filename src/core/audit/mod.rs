//! core/audit — 审计与计费（spec §4.8）

pub mod counter;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 审计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub route_id: String,
    pub api_key_hash: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub latency_ms: u64,
    pub status: u16,
    /// 请求追踪 ID
    pub trace_id: String,
    /// 用户标识（可选）
    pub user_id: Option<String>,
    /// 上游提供商
    pub provider: String,
    /// 使用的模型
    pub model: String,
    /// prompt token 数（u32 兼容 SSE 解析）
    pub prompt_tokens: u32,
    /// completion token 数（u32 兼容 SSE 解析）
    pub completion_tokens: u32,
}

impl AuditEvent {
    pub fn new(
        route_id: String,
        api_key_hash: String,
        tokens_in: u64,
        tokens_out: u64,
        latency_ms: u64,
        status: u16,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            route_id,
            api_key_hash,
            tokens_in,
            tokens_out,
            latency_ms,
            status,
            trace_id: String::new(),
            user_id: None,
            provider: String::new(),
            model: String::new(),
            prompt_tokens: tokens_in as u32,
            completion_tokens: tokens_out as u32,
        }
    }
}
