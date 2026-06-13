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
