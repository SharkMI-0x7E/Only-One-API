//! OAuth2 认证流程（spec §4.5 / stage3 维度 6）
//!
//! 提供 outbound token 获取能力（authorization_code + client_credentials），
//! **不**实现 `Authenticator` trait（那是验证入站凭据的）。
//! Token 缓存使用 `moka`，key 包含 client_id + scope 避免不同客户端共享。
//!
//! **例外**：此模块允许在 core 层使用 reqwest，因为需要向 OAuth2 provider 发起 HTTP 请求。

use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};
use tracing;

use crate::core::error::CoreError;

/// OAuth2 配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: Option<String>,
    /// 默认 scope（可在请求时覆盖）
    #[serde(default)]
    pub default_scopes: Vec<String>,
    /// token 缓存 TTL（秒），默认 300
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
}

fn default_cache_ttl_secs() -> u64 {
    300
}

/// 缓存的 token
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Option<std::time::Instant>,
    refresh_token: Option<String>,
}

impl CachedToken {
    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => std::time::Instant::now() >= exp,
            None => false,
        }
    }
}

/// OAuth2 客户端，封装 authorization_code 与 client_credentials 流程
pub struct OAuth2Client {
    client: BasicClient,
    cache: Cache<String, CachedToken>,
    config: OAuth2Config,
}

impl OAuth2Client {
    /// 从配置构建 OAuth2 客户端
    pub fn new(config: OAuth2Config) -> Result<Self, CoreError> {
        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            AuthUrl::new(config.auth_url.clone())
                .map_err(|e| CoreError::Config(format!("invalid auth_url: {e}")))?,
            Some(
                TokenUrl::new(config.token_url.clone())
                    .map_err(|e| CoreError::Config(format!("invalid token_url: {e}")))?,
            ),
        );

        let client = if let Some(ref redirect) = config.redirect_url {
            client.set_redirect_uri(
                RedirectUrl::new(redirect.clone())
                    .map_err(|e| CoreError::Config(format!("invalid redirect_url: {e}")))?,
            )
        } else {
            client
        };

        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(config.cache_ttl_secs))
            .max_capacity(1000)
            .build();

        Ok(Self {
            client,
            cache,
            config,
        })
    }

    /// 生成 authorization_code 流程的授权 URL
    ///
    /// 返回 (url, csrf_secret)：调用方**必须**存储 csrf_secret，
    /// 在回调时传给 `exchange_authorization_code` 做 state 校验。
    pub fn authorization_url(&self, scopes: Option<&[String]>) -> (String, String) {
        let mut req = self.client.authorize_url(oauth2::CsrfToken::new_random);

        let scope_list = scopes.unwrap_or(&self.config.default_scopes);
        for s in scope_list {
            req = req.add_scope(Scope::new(s.clone()));
        }

        let (url, csrf) = req.url();
        (url.to_string(), csrf.secret().clone())
    }

    /// authorization_code 流程：用授权码换取 token
    ///
    /// `expected_state` 必须与 `authorization_url` 返回的 csrf_secret 一致，
    /// 否则返回 `CoreError::Auth`（防 CSRF 攻击）。
    pub async fn exchange_authorization_code(
        &self,
        code: &str,
        expected_state: &str,
        actual_state: &str,
    ) -> Result<String, CoreError> {
        // 常量时间比较 state，防止 timing side-channel
        use subtle::ConstantTimeEq;
        let expected = expected_state.as_bytes();
        let actual = actual_state.as_bytes();
        if expected.len() != actual.len() || !bool::from(expected.ct_eq(actual)) {
            return Err(CoreError::Auth("oauth2 state mismatch".to_string()));
        }

        let token_result = self
            .client
            .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| CoreError::Auth(format!("oauth2 code exchange failed: {e}")))?;

        let access_token = token_result.access_token().secret().clone();

        let cache_key = self.build_cache_key(&self.config.default_scopes);
        let cached = CachedToken {
            access_token: access_token.clone(),
            expires_at: token_result
                .expires_in()
                .map(|d| std::time::Instant::now() + d),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
        };

        self.cache.insert(cache_key, cached).await;

        tracing::info!(
            flow = "authorization_code",
            client_id = %self.config.client_id,
            "token acquired"
        );

        Ok(access_token)
    }

    /// client_credentials 流程：用 client_id/secret 直接获取 token
    pub async fn client_credentials(&self, scopes: Option<&[String]>) -> Result<String, CoreError> {
        let scope_list = scopes.unwrap_or(&self.config.default_scopes);
        let cache_key = self.build_cache_key(scope_list);

        // 先查缓存
        if let Some(cached) = self.cache.get(&cache_key).await {
            if !cached.is_expired() {
                tracing::debug!(
                    flow = "client_credentials",
                    client_id = %self.config.client_id,
                    "cache hit"
                );
                return Ok(cached.access_token);
            }
        }

        let mut req = self.client.exchange_client_credentials();
        for s in scope_list {
            req = req.add_scope(Scope::new(s.clone()));
        }

        let token_result = req
            .request_async(async_http_client)
            .await
            .map_err(|e| CoreError::Auth(format!("oauth2 client_credentials failed: {e}")))?;

        let access_token = token_result.access_token().secret().clone();

        let cached = CachedToken {
            access_token: access_token.clone(),
            expires_at: token_result
                .expires_in()
                .map(|d| std::time::Instant::now() + d),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
        };

        self.cache.insert(cache_key, cached).await;

        tracing::info!(
            flow = "client_credentials",
            client_id = %self.config.client_id,
            "token acquired"
        );

        Ok(access_token)
    }

    /// 刷新 token（使用 refresh_token）
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        scopes: Option<&[String]>,
    ) -> Result<String, CoreError> {
        let scope_list = scopes.unwrap_or(&self.config.default_scopes);
        let cache_key = self.build_cache_key(scope_list);

        let rt = oauth2::RefreshToken::new(refresh_token.to_string());
        let mut req = self.client.exchange_refresh_token(&rt);

        for s in scope_list {
            req = req.add_scope(Scope::new(s.clone()));
        }

        let token_result = req
            .request_async(async_http_client)
            .await
            .map_err(|e| CoreError::Auth(format!("oauth2 token refresh failed: {e}")))?;

        let access_token = token_result.access_token().secret().clone();

        let cached = CachedToken {
            access_token: access_token.clone(),
            expires_at: token_result
                .expires_in()
                .map(|d| std::time::Instant::now() + d),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
        };

        self.cache.insert(cache_key, cached).await;

        tracing::info!(
            flow = "refresh",
            client_id = %self.config.client_id,
            "token refreshed"
        );

        Ok(access_token)
    }

    /// 获取 token，自动处理缓存与刷新
    ///
    /// 优先返回缓存 token；若过期且有 refresh_token 则自动刷新；
    /// 否则走 client_credentials 重新获取。
    pub async fn get_token(&self, scopes: Option<&[String]>) -> Result<String, CoreError> {
        let scope_list = scopes.unwrap_or(&self.config.default_scopes);
        let cache_key = self.build_cache_key(scope_list);

        if let Some(cached) = self.cache.get(&cache_key).await {
            if !cached.is_expired() {
                tracing::debug!(
                    flow = "auto",
                    client_id = %self.config.client_id,
                    "cache hit"
                );
                return Ok(cached.access_token);
            }

            // 过期但有 refresh_token
            if let Some(ref rt) = cached.refresh_token {
                return self.refresh_token(rt, Some(scope_list)).await;
            }
        }

        // 无缓存或无法刷新，走 client_credentials
        self.client_credentials(Some(scope_list)).await
    }

    /// 构建缓存 key：client_id + sorted scopes
    fn build_cache_key(&self, scopes: &[String]) -> String {
        let mut sorted = scopes.to_vec();
        sorted.sort();
        format!("{}:{}", self.config.client_id, sorted.join(","))
    }
}

/// 便捷构造函数：从配置创建 Arc<OAuth2Client>
pub fn create_oauth2_client(config: OAuth2Config) -> Result<Arc<OAuth2Client>, CoreError> {
    OAuth2Client::new(config).map(Arc::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OAuth2Config {
        OAuth2Config {
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            auth_url: "https://example.com/oauth/authorize".to_string(),
            token_url: "https://example.com/oauth/token".to_string(),
            redirect_url: Some("http://localhost:8080/callback".to_string()),
            default_scopes: vec!["read".to_string(), "write".to_string()],
            cache_ttl_secs: 300,
        }
    }

    #[test]
    fn build_client_success() {
        let config = test_config();
        let client = OAuth2Client::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn invalid_auth_url_rejected() {
        let mut config = test_config();
        config.auth_url = "not a valid url with spaces and \0 null".to_string();
        let client = OAuth2Client::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn invalid_token_url_rejected() {
        let mut config = test_config();
        config.token_url = "not a valid url with spaces and \0 null".to_string();
        let client = OAuth2Client::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn cache_key_includes_client_id_and_scopes() {
        let config = test_config();
        let client = OAuth2Client::new(config).unwrap();
        let key = client.build_cache_key(&["write".to_string(), "read".to_string()]);
        // scopes 排序后应一致
        assert_eq!(key, "test-client:read,write");
    }

    #[test]
    fn cache_key_different_for_different_scopes() {
        let config = test_config();
        let client = OAuth2Client::new(config).unwrap();
        let key1 = client.build_cache_key(&["read".to_string()]);
        let key2 = client.build_cache_key(&["write".to_string()]);
        assert_ne!(key1, key2);
    }

    #[test]
    fn authorization_url_contains_client_id() {
        let config = test_config();
        let client = OAuth2Client::new(config).unwrap();
        let (url, _csrf) = client.authorization_url(None);
        assert!(url.contains("client_id=test-client"));
        assert!(url.contains("example.com/oauth/authorize"));
    }

    #[test]
    fn authorization_url_includes_scopes() {
        let config = test_config();
        let client = OAuth2Client::new(config).unwrap();
        let custom_scopes = vec!["admin".to_string()];
        let (url, _) = client.authorization_url(Some(&custom_scopes));
        assert!(url.contains("scope=admin"));
    }

    #[tokio::test]
    async fn exchange_authorization_code_state_mismatch() {
        let config = test_config();
        let client = OAuth2Client::new(config).unwrap();
        let result = client
            .exchange_authorization_code("code", "expected", "actual")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("state mismatch"));
    }

    #[test]
    fn cached_token_expiry_detection() {
        let token = CachedToken {
            access_token: "tok".to_string(),
            expires_at: Some(std::time::Instant::now() - Duration::from_secs(1)),
            refresh_token: None,
        };
        assert!(token.is_expired());

        let valid_token = CachedToken {
            access_token: "tok".to_string(),
            expires_at: Some(std::time::Instant::now() + Duration::from_secs(3600)),
            refresh_token: None,
        };
        assert!(!valid_token.is_expired());
    }

    #[test]
    fn cached_token_no_expiry_never_expires() {
        let token = CachedToken {
            access_token: "tok".to_string(),
            expires_at: None,
            refresh_token: None,
        };
        assert!(!token.is_expired());
    }

    #[tokio::test]
    async fn cache_insert_and_retrieve() {
        let config = test_config();
        let client = OAuth2Client::new(config).unwrap();
        let cache_key = client.build_cache_key(&["read".to_string()]);
        let cached = CachedToken {
            access_token: "cached-token".to_string(),
            expires_at: Some(std::time::Instant::now() + Duration::from_secs(3600)),
            refresh_token: None,
        };
        client.cache.insert(cache_key.clone(), cached).await;

        let retrieved = client.cache.get(&cache_key).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().access_token, "cached-token");
    }

    #[test]
    fn create_oauth2_client_returns_arc() {
        let config = test_config();
        let client = create_oauth2_client(config);
        assert!(client.is_ok());
    }

    #[test]
    fn config_deserialize() {
        let yaml = r#"
client_id: "my-client"
client_secret: "my-secret"
auth_url: "https://auth.example.com/authorize"
token_url: "https://auth.example.com/token"
redirect_url: "http://localhost/callback"
default_scopes:
  - "read"
  - "write"
cache_ttl_secs: 600
"#;
        let config: OAuth2Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.client_id, "my-client");
        assert_eq!(config.cache_ttl_secs, 600);
        assert_eq!(config.default_scopes.len(), 2);
    }

    #[test]
    fn config_deserialize_defaults() {
        let yaml = r#"
client_id: "c"
client_secret: "s"
auth_url: "https://a.com/auth"
token_url: "https://a.com/token"
"#;
        let config: OAuth2Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.cache_ttl_secs, 300);
        assert!(config.default_scopes.is_empty());
        assert!(config.redirect_url.is_none());
    }
}
