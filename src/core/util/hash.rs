//! 哈希工具（spec §4.7）
//!
//! 阶段一只实现普通哈希；一致性哈希留 [S2]。

use sha2::{Digest, Sha256};

/// 对任意字节做 SHA-256，返回 32 字节摘要
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

/// `u64` 非加密哈希（用于一致性 hash 桶选择）
pub fn bucket_index(key: &str, buckets: usize) -> usize {
    debug_assert!(buckets > 0, "buckets must be positive");
    let h = sha256(key.as_bytes());
    let mut v: u64 = 0;
    for (i, b) in h.iter().take(8).enumerate() {
        v |= (*b as u64) << (i * 8);
    }
    (v as usize) % buckets
}

/// API Key 的 SHA-256 十六进制表示（用于日志脱敏）
pub fn fingerprint_api_key(api_key: &str) -> String {
    let h = sha256(api_key.as_bytes());
    hex::encode_upper(&h[..8])
}

mod hex {
    /// 不依赖 hex crate，自实现前 N 字节十六进制编码（大写）
    pub fn encode_upper(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(HEX[(b >> 4) as usize] as char);
            s.push(HEX[(b & 0x0F) as usize] as char);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_index_within_range() {
        for key in ["alpha", "beta", "gamma", "delta", "epsilon"] {
            let b = bucket_index(key, 7);
            assert!(b < 7);
        }
    }

    #[test]
    fn fingerprint_is_deterministic_and_short() {
        let a = fingerprint_api_key("sk-test-1234567890");
        let b = fingerprint_api_key("sk-test-1234567890");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }
}
