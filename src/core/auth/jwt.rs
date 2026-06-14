//! JWT 认证（spec §4.5）
//!
//! 支持 HS256 / RS256，从 `Authorization: Bearer <token>` 提取并验证。

use async_trait::async_trait;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::core::auth::Authenticator;
use crate::core::error::CoreError;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Option<String>,
    iss: Option<String>,
    exp: Option<u64>,
}

pub struct JwtAuthenticator {
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    issuer: Option<String>,
}

impl JwtAuthenticator {
    pub fn new_hs256(secret: &[u8], issuer: Option<String>) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret),
            algorithm: Algorithm::HS256,
            issuer,
        }
    }

    pub fn new_rs256(pem: &[u8], issuer: Option<String>) -> Result<Self, CoreError> {
        let key = DecodingKey::from_rsa_pem(pem)
            .map_err(|e| CoreError::Config(format!("invalid RSA key: {e}")))?;
        Ok(Self {
            decoding_key: key,
            algorithm: Algorithm::RS256,
            issuer,
        })
    }
}

#[async_trait]
impl Authenticator for JwtAuthenticator {
    async fn verify(&self, credential: &str) -> Result<(), CoreError> {
        let mut validation = Validation::new(self.algorithm);
        validation.validate_exp = true;
        if let Some(ref iss) = self.issuer {
            validation.set_issuer(&[iss]);
        }
        validation.validate_aud = false;

        let _token_data = decode::<Claims>(credential, &self.decoding_key, &validation)
            .map_err(|e| CoreError::Auth(format!("jwt verify failed: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[tokio::test]
    async fn hs256_valid_token() {
        let secret = b"test-secret-key-for-testing-1234";
        let auth = JwtAuthenticator::new_hs256(secret, None);
        let claims = Claims {
            sub: Some("user1".into()),
            iss: None,
            exp: Some(chrono::Utc::now().timestamp() as u64 + 3600),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();
        assert!(auth.verify(&token).await.is_ok());
    }

    #[tokio::test]
    async fn hs256_expired_token() {
        let secret = b"test-secret-key-for-testing-1234";
        let auth = JwtAuthenticator::new_hs256(secret, None);
        let claims = Claims {
            sub: Some("user1".into()),
            iss: None,
            exp: Some(1000),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap();
        assert!(auth.verify(&token).await.is_err());
    }

    #[tokio::test]
    async fn wrong_secret_rejected() {
        let claims = Claims {
            sub: Some("user1".into()),
            iss: None,
            exp: Some(chrono::Utc::now().timestamp() as u64 + 3600),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"secret-a"),
        )
        .unwrap();
        let auth = JwtAuthenticator::new_hs256(b"secret-b", None);
        assert!(auth.verify(&token).await.is_err());
    }
}
