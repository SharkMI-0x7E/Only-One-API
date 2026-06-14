//! core/auth — 认证抽象（spec §4.5）

pub mod apikey;
pub mod jwt;

use async_trait::async_trait;

use crate::core::error::CoreError;

/// 通用认证器
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// 验证凭据；返回 Ok(()) 通过 / Err(CoreError::Auth) 失败
    ///
    /// **错误响应必须统一为 `unauthorized`，禁止区分"key 不存在" vs "key 错误"**
    async fn verify(&self, credential: &str) -> Result<(), CoreError>;
}
