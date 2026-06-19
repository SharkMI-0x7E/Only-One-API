//! service/upstream_pool — reqwest::Client pool + SSRF implementation (spec §5 + §8)
//!
//! Stage 2 enhancements: DNS resolution + IP range checking + per-upstream timeout
//! and configurable connection pool parameters.

use std::net::{IpAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use crate::core::config::upstream::UpstreamConfig;
use crate::core::error::CoreError;
use crate::service::state::UpstreamCache;

/// Default pool idle timeout in seconds
const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 90;

/// Default max idle connections per host
const DEFAULT_POOL_MAX_IDLE_PER_HOST: usize = 10;

/// Default TCP keepalive in seconds
const DEFAULT_TCP_KEEPALIVE_SECS: u64 = 60;

/// Upstream connection pool with SSRF protection
pub struct UpstreamPool {
    cache: UpstreamCache,
    allowlist: Arc<Vec<String>>,
    default_request_timeout: Duration,
    max_body_bytes: usize,
}

impl UpstreamPool {
    /// Create a new upstream pool
    pub fn new(allowlist: Vec<String>, request_timeout_ms: u64, max_body_bytes: usize) -> Self {
        Self {
            cache: UpstreamCache::builder().max_capacity(1024).build(),
            allowlist: Arc::new(allowlist),
            default_request_timeout: Duration::from_millis(request_timeout_ms),
            max_body_bytes,
        }
    }

    /// Check if base_url is in the allowlist
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

    /// Full SSRF check: DNS resolution + IP range check
    pub fn check_ssrf(&self, base_url: &str) -> Result<(), CoreError> {
        self.check_allowlist(base_url)?;

        let host = base_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .ok_or_else(|| CoreError::BadRequest(format!("invalid base_url: {base_url}")))?;

        // DNS resolution
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

    /// Get (or create and cache) the reqwest::Client for the given upstream
    pub async fn client_for(&self, up: &UpstreamConfig) -> Result<Arc<reqwest::Client>, CoreError> {
        self.check_ssrf(&up.base_url)?;
        if let Some(c) = self.cache.get(&up.id).await {
            return Ok(c);
        }

        // Use per-upstream timeout if specified, otherwise fall back to global default
        let timeout = up
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(self.default_request_timeout);

        // Use per-upstream pool config if specified, otherwise use defaults
        let pool_config = up.pool.as_ref();
        let idle_timeout = pool_config
            .map(|p| Duration::from_secs(p.idle_timeout_secs))
            .unwrap_or(Duration::from_secs(DEFAULT_POOL_IDLE_TIMEOUT_SECS));
        let max_idle_per_host = pool_config
            .map(|p| p.max_idle_per_host)
            .unwrap_or(DEFAULT_POOL_MAX_IDLE_PER_HOST);

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .pool_idle_timeout(idle_timeout)
            .pool_max_idle_per_host(max_idle_per_host)
            .tcp_keepalive(Duration::from_secs(DEFAULT_TCP_KEEPALIVE_SECS))
            .user_agent(concat!("rapidgate/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::UpstreamUnreachable(format!("client build: {e}")))?;

        let arc = Arc::new(client);
        self.cache.insert(up.id.clone(), arc.clone()).await;
        Ok(arc)
    }

    /// Get the maximum allowed request body size
    pub fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
    }

    /// Remove a cached client for the given upstream ID
    pub async fn remove(&self, upstream_id: &str) {
        self.cache.remove(upstream_id).await;
    }

    /// Perform a health check on an upstream by sending a HEAD request to its base_url
    pub async fn health_check(&self, up: &UpstreamConfig) -> Result<(), CoreError> {
        let client = self.client_for(up).await?;
        let health_url = format!("{}/health", up.base_url.trim_end_matches('/'));

        client
            .head(&health_url)
            .send()
            .await
            .map_err(|e| CoreError::UpstreamUnreachable(format!("health check failed: {e}")))?;

        Ok(())
    }
}

/// Check if IP is in private/loopback/link-local ranges
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
