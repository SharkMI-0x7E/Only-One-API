//! 认证集成测试（spec §5.3）

use rapidgate::core::auth::apikey::ApiKeyAuthenticator;
use rapidgate::core::auth::jwt::JwtAuthenticator;
use rapidgate::core::auth::Authenticator;

#[tokio::test]
async fn api_key_accepts_registered() {
    let auth = ApiKeyAuthenticator::new();
    auth.register("sk-test-1234567890".to_string());
    assert!(auth.verify("sk-test-1234567890").await.is_ok());
}

#[tokio::test]
async fn api_key_rejects_unknown() {
    let auth = ApiKeyAuthenticator::new();
    auth.register("sk-test-1234567890".to_string());
    assert!(auth.verify("sk-wrong-key-0000000000").await.is_err());
}

#[tokio::test]
async fn jwt_hs256_valid() {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: u64,
    }

    let secret = b"integration-test-secret-key-1234";
    let claims = Claims {
        sub: "user1".to_string(),
        exp: chrono::Utc::now().timestamp() as u64 + 3600,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    let auth = JwtAuthenticator::new_hs256(secret, None);
    assert!(auth.verify(&token).await.is_ok());
}

#[tokio::test]
async fn jwt_hs256_expired() {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: u64,
    }

    let secret = b"integration-test-secret-key-1234";
    let claims = Claims {
        sub: "user1".to_string(),
        exp: 1000, // 已过期
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    let auth = JwtAuthenticator::new_hs256(secret, None);
    assert!(auth.verify(&token).await.is_err());
}
