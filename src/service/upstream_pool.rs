//! service/upstream_pool — reqwest::Client 池 + SSRF 完整实现（spec §5 + §8）
//!
//! 阶段二增强：DNS 解析 + IP 段检查 + 连接池调参

use std::net::{IpAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use crate::core::config::upstream::UpstreamConfig;
use crate::core::error::CoreError;
use crate::service::state::UpstreamCache;

pub struct UpstreamPool {
    cache: UpstreamCache,
    allowlist: Arc<Vec<String>>,
    request_timeout: Duration,
    max_body_bytes: usize,
}

impl UpstreamPool {
    pub fn new(allowlist: Vec<String>, request_timeout_ms: u64, max_body_bytes: usize) -> Self {
        Self {
            cache: UpstreamCache::builder().max_capacity(1024).build(),
            allowlist: Arc::new(allowlist),
            request_timeout: Duration::from_millis(request_timeout_ms),
            max_body_bytes,
        }
    }

    /// 检查 base_url 是否在 allowlist
    pub fn check_allowlist(&self, base_url: &str) -> Result<(), CoreError> {
        let host = base_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.split(':').next())
            .ok_or_else(|| CoreError::BadRequest(format!("invalid base_url: {base_url}")))?;
        if self.allowlist.iter().any(|h| h.eq_ignore_ascii_case(host)) {
            Ok(())
        } else {
            Err(CoreError::BadRequest(format!(
                "upstream host '{host}' not in allowlist"
            )))
        }
    }

    /// SSRF 完整检查：DNS 解析 + IP 段检查
    pub fn check_ssrf(&self, base_url: &str) -> Result<(), CoreError> {
        self.check_allowlist(base_url)?;

        let host = base_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .ok_or_else(|| CoreError::BadRequest(format!("invalid base_url: {base_url}")))?;

        // DNS 解析
        let addr_str = if host.contains(':') {
            host.to_string()
        } else {
            format!("{host}:443")
        };

        let addrs: Vec<_> = match addr_str.to_socket_addrs() {
            Ok(addrs) => addrs.collect(),
            Err(e) => {
                return Err(CoreError::BadRequest(format!(
                    "DNS resolution failed for '{host}': {e}"
                )));
            }
        };

        for addr in &addrs {
            let ip = addr.ip();
            if is_blocked_ip(ip) {
                return Err(CoreError::BadRequest(format!(
                    "upstream '{host}' resolved to blocked IP: {ip}"
                )));
            }
        }

        Ok(())
    }

    /// 拿到（或构造并缓存）该 upstream 对应的 reqwest::Client
    pub async fn client_for(&self, up: &UpstreamConfig) -> Result<Arc<reqwest::Client>, CoreError> {
        self.check_ssrf(&up.base_url)?;
        if let Some(c) = self.cache.get(&up.id).await {
            return Ok(c);
        }
        let client = reqwest::Client::builder()
            .timeout(self.request_timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .user_agent(concat!("rapidgate/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::UpstreamUnreachable(format!("client build: {e}")))?;
        let arc = Arc::new(client);
        self.cache.insert(up.id.clone(), arc.clone()).await;
        Ok(arc)
    }

    pub fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
    }
}

/// 检查 IP 是否在私有/回环/链路本地段
fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || is_in_cidr_127(&v4)
                || is_in_cidr_10(&v4)
                || is_in_cidr_172_16(&v4)
                || is_in_cidr_192_168(&v4)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || is_in_cidr_fc00(&v6) || is_in_cidr_fe80(&v6)
        }
    }
}

fn is_in_cidr_127(ip: &std::net::Ipv4Addr) -> bool {
    ip.octets()[0] == 127
}

fn is_in_cidr_10(ip: &std::net::Ipv4Addr) -> bool {
    ip.octets()[0] == 10
}

fn is_in_cidr_172_16(ip: &std::net::Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 172 && (16..=31).contains(&o[1])
}

fn is_in_cidr_192_168(ip: &std::net::Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 192 && o[1] == 168
}

fn is_in_cidr_fc00(ip: &std::net::Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn is_in_cidr_fe80(ip: &std::net::Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_loopback_v4() {
        assert!(is_blocked_ip("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn blocks_private_10() {
        assert!(is_blocked_ip("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn blocks_private_172() {
        assert!(is_blocked_ip("172.16.0.1".parse().unwrap()));
        assert!(is_blocked_ip("172.31.255.255".parse().unwrap()));
    }

    #[test]
    fn blocks_private_192() {
        assert!(is_blocked_ip("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn blocks_link_local() {
        assert!(is_blocked_ip("169.254.1.1".parse().unwrap()));
    }

    #[test]
    fn allows_public_ip() {
        assert!(!is_blocked_ip("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn blocks_loopback_v6() {
        assert!(is_blocked_ip("::1".parse().unwrap()));
    }
}
