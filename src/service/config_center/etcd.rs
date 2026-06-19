//! ETCD configuration center backend
//!
//! Note: The `etcd-client` dependency is currently disabled because it requires
//! the `protoc` compiler. To enable: uncomment `etcd-client` in `Cargo.toml`,
//! install protoc, and recompile.

use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::Stream;

use super::ConfigCenter;
use crate::core::error::CoreError;

/// ETCD connection configuration
#[derive(Debug, Clone)]
pub struct EtcdConfig {
    /// ETCD endpoint list
    pub endpoints: Vec<String>,
    /// Optional username for authentication
    pub username: Option<String>,
    /// Optional password for authentication
    pub password: Option<String>,
    /// Configuration prefix (default: "/rapidgate/config")
    pub prefix: String,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://localhost:2379".to_string()],
            username: None,
            password: None,
            prefix: "/rapidgate/config".to_string(),
        }
    }
}

/// ETCD configuration center
pub struct EtcdConfigCenter {
    config: EtcdConfig,
}

impl EtcdConfigCenter {
    /// Create an ETCD configuration center instance
    pub fn new(config: EtcdConfig) -> Self {
        tracing::warn!("ETCD config center created but etcd-client is not compiled in; operations will return errors");
        Self { config }
    }

    /// Get reference to ETCD configuration
    pub fn config(&self) -> &EtcdConfig {
        &self.config
    }
}

#[async_trait]
impl ConfigCenter for EtcdConfigCenter {
    async fn fetch(&self, key: &str) -> Result<String, CoreError> {
        let full_key = format!("{}/{}", self.config.prefix, key);
        tracing::warn!(key = %full_key, "ETCD fetch attempted but etcd-client is not compiled in");
        Err(CoreError::Config(
            "ETCD support requires etcd-client feature (needs protoc installed)".to_string(),
        ))
    }

    async fn watch(
        &self,
        key: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>, CoreError> {
        tracing::warn!(key = %key, "ETCD watch attempted but etcd-client is not compiled in");
        Err(CoreError::Config(
            "ETCD support requires etcd-client feature (needs protoc installed)".to_string(),
        ))
    }
}
