//! Consul configuration center backend
//!
//! Provides configuration fetching from Consul KV store.
//! Supports both single key fetch and watch for changes.

use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::Stream;

use super::ConfigCenter;
use crate::core::error::CoreError;

/// Consul connection configuration
#[derive(Debug, Clone)]
pub struct ConsulConfig {
    /// Consul HTTP endpoint (default: "http://localhost:8500")
    pub endpoint: String,
    /// Optional datacenter name
    pub datacenter: Option<String>,
    /// Optional ACL token for authentication
    pub token: Option<String>,
    /// Configuration prefix (default: "rapidgate/config")
    pub prefix: String,
}

impl Default for ConsulConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8500".to_string(),
            datacenter: None,
            token: None,
            prefix: "rapidgate/config".to_string(),
        }
    }
}

/// Consul configuration center
pub struct ConsulConfigCenter {
    config: ConsulConfig,
    client: reqwest::Client,
}

impl ConsulConfigCenter {
    /// Create a Consul configuration center instance
    pub fn new(config: ConsulConfig) -> Self {
        let client = reqwest::Client::new();
        tracing::info!(endpoint = %config.endpoint, prefix = %config.prefix, "Consul config center initialized");
        Self { config, client }
    }

    /// Get reference to Consul configuration
    pub fn config(&self) -> &ConsulConfig {
        &self.config
    }

    /// Build the full key path
    fn full_key(&self, key: &str) -> String {
        format!("{}/{}", self.config.prefix, key)
    }

    /// Build the Consul API URL for a key
    fn build_url(&self, key: &str) -> String {
        format!("{}/v1/kv/{}", self.config.endpoint, key)
    }
}

#[async_trait]
impl ConfigCenter for ConsulConfigCenter {
    async fn fetch(&self, key: &str) -> Result<String, CoreError> {
        let full_key = self.full_key(key);
        let url = self.build_url(&full_key);

        let mut req = self.client.get(&url);

        // Add optional headers
        if let Some(ref dc) = self.config.datacenter {
            req = req.query(&[("dc", dc)]);
        }
        if let Some(ref token) = self.config.token {
            req = req.header("X-Consul-Token", token);
        }

        let resp = req.send().await.map_err(|e| {
            tracing::error!(key = %full_key, error = %e, "Consul fetch failed");
            CoreError::Config(format!("Consul fetch failed: {}", e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(key = %full_key, status = %status, body = %body, "Consul fetch failed");
            return Err(CoreError::Config(format!(
                "Consul fetch failed with status {}: {}",
                status, body
            )));
        }

        // Consul returns JSON array with base64-encoded Value
        let entries: Vec<ConsulKVEntry> = resp.json().await.map_err(|e| {
            tracing::error!(key = %full_key, error = %e, "Consul response parse failed");
            CoreError::Config(format!("Consul response parse failed: {}", e))
        })?;

        if entries.is_empty() {
            return Err(CoreError::Config(format!(
                "Consul key not found: {}",
                full_key
            )));
        }

        // Decode base64 value
        let value = entries[0]
            .decode_value()
            .map_err(|e| CoreError::Config(format!("Consul value decode failed: {}", e)))?;

        tracing::info!(key = %full_key, "Consul fetch succeeded");
        Ok(value)
    }

    async fn watch(
        &self,
        key: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>, CoreError> {
        // Consul blocking queries can be implemented here
        // For now, return an error indicating watch is not yet implemented
        tracing::warn!(key = %key, "Consul watch not yet implemented");
        Err(CoreError::Config(
            "Consul watch not yet implemented; use fetch() for one-time reads".to_string(),
        ))
    }
}

/// Consul KV entry structure (partial, for JSON parsing)
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct ConsulKVEntry {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: Option<String>,
    #[serde(rename = "ModifyIndex")]
    modify_index: u64,
}

impl ConsulKVEntry {
    /// Decode the base64-encoded value
    fn decode_value(&self) -> Result<String, String> {
        match &self.value {
            Some(v) => {
                let decoded =
                    base64_decode(v).map_err(|e| format!("base64 decode failed: {}", e))?;
                String::from_utf8(decoded).map_err(|e| format!("UTF-8 decode failed: {}", e))
            }
            None => Ok(String::new()),
        }
    }
}

/// Simple base64 decoder (standard alphabet)
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const DECODE_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input = input.trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    let mut buffer = 0u32;
    let mut bits_collected = 0;

    for c in input.chars() {
        let digit = DECODE_TABLE
            .iter()
            .position(|&d| d as char == c)
            .ok_or_else(|| format!("invalid base64 character: {}", c))? as u32;

        buffer = (buffer << 6) | digit;
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}
