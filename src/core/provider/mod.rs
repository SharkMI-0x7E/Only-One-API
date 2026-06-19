//! core/provider — Provider registry abstraction
//!
//! Defines the `Provider` trait and `ProviderRegistry` for managing multiple LLM providers.
//! All provider implementations must implement the `Provider` trait, and the registry
//! provides lookup and management capabilities.

use std::collections::HashMap;
use std::sync::Arc;

use crate::core::config::ProviderKind;

/// Provider trait
///
/// All LLM providers must implement this trait, which defines the core capabilities
/// for request forwarding and model management.
pub trait Provider: Send + Sync {
    /// Get the provider's unique identifier
    fn id(&self) -> &str;

    /// Get the provider type
    fn kind(&self) -> ProviderKind;

    /// Get the base URL for API requests
    fn base_url(&self) -> &str;

    /// Get the API key for authentication
    fn api_key(&self) -> &str;

    /// Check if this provider supports the given model
    fn supports_model(&self, model: &str) -> bool;

    /// List all models supported by this provider
    fn supported_models(&self) -> &[String];
}

/// Provider registry
///
/// Manages multiple provider instances and provides lookup capabilities.
/// Thread-safe and can be shared across requests via `Arc<ProviderRegistry>`.
pub struct ProviderRegistry {
    /// Provider storage, keyed by provider ID
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// Create an empty provider registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider
    ///
    /// If a provider with the same ID already exists, it will be replaced.
    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        let id = provider.id().to_string();
        tracing::debug!(provider_id = %id, kind = %provider.kind(), "registering provider");
        self.providers.insert(id, provider);
    }

    /// Unregister a provider by ID
    pub fn unregister(&mut self, provider_id: &str) -> Option<Arc<dyn Provider>> {
        tracing::debug!(provider_id = %provider_id, "unregistering provider");
        self.providers.remove(provider_id)
    }

    /// Get a provider by ID
    pub fn get(&self, provider_id: &str) -> Option<&Arc<dyn Provider>> {
        self.providers.get(provider_id)
    }

    /// List all registered providers
    pub fn list(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.providers.values()
    }

    /// Get the number of registered providers
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Find all providers that support the given model
    pub fn find_by_model(&self, model: &str) -> Vec<&Arc<dyn Provider>> {
        self.providers
            .values()
            .filter(|p| p.supports_model(model))
            .collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock provider for testing purposes
    struct MockProvider {
        id: String,
        kind: ProviderKind,
        base_url: String,
        api_key: String,
        models: Vec<String>,
    }

    impl MockProvider {
        fn new(
            id: impl Into<String>,
            kind: ProviderKind,
            base_url: impl Into<String>,
            api_key: impl Into<String>,
            models: Vec<String>,
        ) -> Self {
            Self {
                id: id.into(),
                kind,
                base_url: base_url.into(),
                api_key: api_key.into(),
                models,
            }
        }
    }

    impl Provider for MockProvider {
        fn id(&self) -> &str {
            &self.id
        }

        fn kind(&self) -> ProviderKind {
            self.kind
        }

        fn base_url(&self) -> &str {
            &self.base_url
        }

        fn api_key(&self) -> &str {
            &self.api_key
        }

        fn supports_model(&self, model: &str) -> bool {
            self.models.iter().any(|m| m == model)
        }

        fn supported_models(&self) -> &[String] {
            &self.models
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider::new(
            "openai",
            ProviderKind::OpenAI,
            "https://api.openai.com/v1",
            "sk-test",
            vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
        ));

        registry.register(provider.clone());
        assert_eq!(registry.len(), 1);

        let retrieved = registry.get("openai");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "openai");
    }

    #[test]
    fn test_find_by_model() {
        let mut registry = ProviderRegistry::new();

        let openai = Arc::new(MockProvider::new(
            "openai",
            ProviderKind::OpenAI,
            "https://api.openai.com/v1",
            "sk-test",
            vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
        ));

        let anthropic = Arc::new(MockProvider::new(
            "anthropic",
            ProviderKind::Anthropic,
            "https://api.anthropic.com",
            "sk-ant-test",
            vec!["claude-3-opus".to_string(), "gpt-4".to_string()],
        ));

        registry.register(openai);
        registry.register(anthropic);

        let gpt4_providers = registry.find_by_model("gpt-4");
        assert_eq!(gpt4_providers.len(), 2);

        let claude_providers = registry.find_by_model("claude-3-opus");
        assert_eq!(claude_providers.len(), 1);
        assert_eq!(claude_providers[0].id(), "anthropic");

        let unknown_providers = registry.find_by_model("unknown-model");
        assert_eq!(unknown_providers.len(), 0);
    }
}
