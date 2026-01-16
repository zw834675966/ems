//! 认证能力：登录、JWT 生成与校验。

mod jwt;
mod password;

use async_trait::async_trait;
use domain::TenantContext;
use ems_storage::{UserRecord, UserStore};
use std::sync::Arc;

pub use jwt::JwtManager;
pub use password::{PasswordCheck, hash_password, verify_password_and_maybe_upgrade};

/// 认证相关错误。
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("token expired")]
    TokenExpired,
    #[error("token invalid")]
    TokenInvalid,
    #[error("internal error: {0}")]
    Internal(String),
}

/// 登录/刷新返回的 token 结构。
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_jti: String,
    pub expires_at: u64,
}

/// 认证服务实现（基于 UserStore + JWT）。
pub struct AuthService {
    user_store: Arc<dyn UserStore>,
    jwt: JwtManager,
}

impl AuthService {
    /// 创建认证服务实例。
    pub fn new(user_store: Arc<dyn UserStore>, jwt: JwtManager) -> Self {
        Self { user_store, jwt }
    }

    /// 登录校验并签发 token。
    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(UserRecord, AuthTokens), AuthError> {
        let ctx = TenantContext::default();
        let user = self
            .user_store
            .find_by_username(&ctx, username)
            .await
            .map_err(|err| AuthError::Internal(err.to_string()))?
            .ok_or(AuthError::InvalidCredentials)?;
        let check = verify_password_and_maybe_upgrade(&user.password, password)?;
        if !check.verified {
            return Err(AuthError::InvalidCredentials);
        }
        if let Some(password_hash) = check.upgrade_hash {
            let ctx = user.to_tenant_context();
            let updated = self
                .user_store
                .update_password_hash(&ctx, &user.user_id, &password_hash)
                .await
                .map_err(|err| AuthError::Internal(err.to_string()))?;
            if !updated {
                return Err(AuthError::Internal("password migration update failed".to_string()));
            }
        }
        let ctx = user.to_tenant_context();
        let tokens = self.jwt.issue_tokens(&ctx)?;
        let updated = self
            .user_store
            .set_refresh_jti(&ctx, &user.user_id, Some(&tokens.refresh_jti))
            .await
            .map_err(|err| AuthError::Internal(err.to_string()))?;
        if !updated {
            return Err(AuthError::Internal("refresh token binding update failed".to_string()));
        }
        Ok((user, tokens))
    }

    /// 校验 access token 并提取 TenantContext。
    pub fn verify_access_token(&self, token: &str) -> Result<TenantContext, AuthError> {
        self.jwt.decode_access(token)
    }

    /// 使用 refresh token 换取新 token。
    pub async fn refresh(&self, token: &str) -> Result<AuthTokens, AuthError> {
        let (ctx, jti) = self.jwt.decode_refresh_with_jti(token)?;
        let stored = self
            .user_store
            .get_refresh_jti(&ctx, &ctx.user_id)
            .await
            .map_err(|err| AuthError::Internal(err.to_string()))?;
        if stored.as_deref() != Some(jti.as_str()) {
            return Err(AuthError::TokenInvalid);
        }

        let tokens = self.jwt.issue_tokens(&ctx)?;
        let updated = self
            .user_store
            .set_refresh_jti(&ctx, &ctx.user_id, Some(&tokens.refresh_jti))
            .await
            .map_err(|err| AuthError::Internal(err.to_string()))?;
        if !updated {
            return Err(AuthError::Internal("refresh token rotation update failed".to_string()));
        }
        Ok(tokens)
    }
}

/// 认证能力 trait，便于替换实现与测试。
#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(UserRecord, AuthTokens), AuthError>;
    fn verify_access_token(&self, token: &str) -> Result<TenantContext, AuthError>;
    async fn refresh(&self, token: &str) -> Result<AuthTokens, AuthError>;
}

#[async_trait]
impl Authenticator for AuthService {
    async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(UserRecord, AuthTokens), AuthError> {
        self.login(username, password).await
    }

    fn verify_access_token(&self, token: &str) -> Result<TenantContext, AuthError> {
        self.verify_access_token(token)
    }

    async fn refresh(&self, token: &str) -> Result<AuthTokens, AuthError> {
        self.refresh(token).await
    }
}
