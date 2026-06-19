//! service/config_center — Configuration center abstraction (spec §5.8)
//!
//! Supports pulling configuration from ETCD / Consul / file system.
//! Fetched configuration still goes through config_loader's R-1~R-8 validation.

pub mod consul;
pub mod etcd;

use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::Stream;

use crate::core::error::CoreError;
use crate::service::config_loader::{expand_placeholders, validate, LoadedConfig};

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

/// Fetch and validate configuration from a config center
///
/// This function:
/// 1. Fetches YAML text from the config center
/// 2. Expands `${VAR}` placeholders using environment variables
/// 3. Validates the configuration against spec §7 rules (R-1~R-8)
/// 4. Returns the validated configuration
pub async fn fetch_and_validate(
    center: &dyn ConfigCenter,
    key: &str,
) -> Result<LoadedConfig, CoreError> {
    let yaml_text = center.fetch(key).await?;
    let expanded = expand_placeholders(&yaml_text)?;
    let cfg: LoadedConfig = serde_yaml::from_str(&expanded)
        .map_err(|e| CoreError::Config(format!("YAML parse error: {}", e)))?;
    validate(&cfg)?;
    Ok(cfg)
}
