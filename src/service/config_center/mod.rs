//! service/config_center — Configuration center abstraction (spec §5.8)
//!
//! Supports pulling configuration from ETCD / Consul / file system.
//! Fetched configuration still goes through config_loader's R-1~R-8 validation.

pub mod etcd;

use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::Stream;

use crate::core::error::CoreError;

/// Configuration center trait
#[async_trait]
pub trait ConfigCenter: Send + Sync {
    /// Fetch configuration content (YAML text) for the specified key
    async fn fetch(&self, key: &str) -> Result<String, CoreError>;

    /// Watch for configuration changes, returns a stream of change notifications
    async fn watch(
        &self,
        key: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>, CoreError>;
}
