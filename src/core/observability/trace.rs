//! TraceId：32 字节十六进制（spec §4.6）
//!
//! 等价于 W3C tracecontext 16 字节（每个字节两位 hex）展开到 32 字符。

use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceId(pub String);

impl TraceId {
    /// 生成新的 trace id
    pub fn new() -> Self {
        let u = Uuid::new_v4();
        // 16 字节（去掉 4 个连字符）→ 32 字符 hex
        let mut s = u.simple().to_string();
        s.make_ascii_uppercase();
        Self(s)
    }

    /// 从 hex 字符串解析（32 字符）
    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self(s.to_ascii_uppercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// W3C tracecontext（spec §4.6）
///
/// 格式：`{version}-{trace-id}-{parent-id}-{flags}`
/// 例：`00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub flags: u8,
}

impl TraceContext {
    /// 从 W3C traceparent header 解析
    pub fn from_traceparent(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return None;
        }
        if parts[0] != "00" {
            return None;
        }
        if parts[1].len() != 32 || !parts[1].chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        if parts[2].len() != 16 || !parts[2].chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        Some(Self {
            trace_id: parts[1].to_string(),
            span_id: parts[2].to_string(),
            flags,
        })
    }

    /// 生成新的 span id 并返回 traceparent 字符串
    pub fn new_span(&self) -> (Self, String) {
        let new_span = generate_span_id();
        let child = Self {
            trace_id: self.trace_id.clone(),
            span_id: new_span.clone(),
            flags: self.flags,
        };
        let parent = format!("00-{}-{}-{:02x}", self.trace_id, new_span, self.flags);
        (child, parent)
    }

    /// 生成全新的 tracecontext（无上游传播时）
    pub fn new_root() -> (Self, String) {
        let trace_id = TraceId::new();
        let span_id = generate_span_id();
        let ctx = Self {
            trace_id: trace_id.0.clone(),
            span_id: span_id.clone(),
            flags: 0x01,
        };
        let parent = format!("00-{}-{}-01", trace_id.0, span_id);
        (ctx, parent)
    }

    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-{:02x}", self.trace_id, self.span_id, self.flags)
    }
}

fn generate_span_id() -> String {
    let u = uuid::Uuid::new_v4();
    u.simple().to_string()[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_32_char_hex() {
        let id = TraceId::new();
        assert_eq!(id.as_str().len(), 32);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn roundtrip_hex() {
        let id = TraceId::new();
        let parsed = TraceId::from_hex(id.as_str()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(TraceId::from_hex("abcd").is_none());
    }

    #[test]
    fn rejects_non_hex() {
        assert!(TraceId::from_hex(&"z".repeat(32)).is_none());
    }
}
