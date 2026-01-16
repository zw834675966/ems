use crate::{AuthError, AuthTokens};
use domain::TenantContext;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// 区分 access 与 refresh 的 token 类型。
const ACCESS_TOKEN_TYPE: &str = "access";
const REFRESH_TOKEN_TYPE: &str = "refresh";

#[derive(Debug, Serialize, Deserialize)]
/// JWT 内部 claims。
struct Claims {
    sub: String,
    tenant_id: String,
    roles: Vec<String>,
    permissions: Vec<String>,
    exp: usize,
    token_type: String,
    jti: Option<String>,
}

/// JWT 生成与校验。
pub struct JwtManager {
    secret: Vec<u8>,
    access_ttl_seconds: u64,
    refresh_ttl_seconds: u64,
}

impl JwtManager {
    /// 创建 JWT 管理器。
    pub fn new(secret: String, access_ttl_seconds: u64, refresh_ttl_seconds: u64) -> Self {
        Self {
            secret: secret.into_bytes(),
            access_ttl_seconds,
            refresh_ttl_seconds,
        }
    }

    /// 基于 TenantContext 签发 access/refresh token。
    pub fn issue_tokens(&self, ctx: &TenantContext) -> Result<AuthTokens, AuthError> {
        let access_token = self.encode(ctx, self.access_ttl_seconds, ACCESS_TOKEN_TYPE, None)?;
        let refresh_jti = Uuid::new_v4().to_string();
        let refresh_token = self.encode(
            ctx,
            self.refresh_ttl_seconds,
            REFRESH_TOKEN_TYPE,
            Some(refresh_jti.clone()),
        )?;
        let expires_at = now_epoch_seconds() + self.access_ttl_seconds;
        Ok(AuthTokens {
            access_token,
            refresh_token,
            refresh_jti,
            expires_at,
        })
    }

    /// 解析 access token。
    pub fn decode_access(&self, token: &str) -> Result<TenantContext, AuthError> {
        self.decode(token, ACCESS_TOKEN_TYPE)
    }

    /// 解析 refresh token。
    pub fn decode_refresh(&self, token: &str) -> Result<TenantContext, AuthError> {
        self.decode(token, REFRESH_TOKEN_TYPE)
    }

    pub fn decode_refresh_with_jti(&self, token: &str) -> Result<(TenantContext, String), AuthError> {
        let decoded = self.decode_claims(token)?;
        if decoded.token_type != REFRESH_TOKEN_TYPE {
            return Err(AuthError::TokenInvalid);
        }
        let ctx = TenantContext::new(
            decoded.tenant_id,
            decoded.sub,
            decoded.roles,
            decoded.permissions,
            None,
        );
        let jti = decoded.jti.ok_or(AuthError::TokenInvalid)?;
        Ok((ctx, jti))
    }

    /// 内部编码逻辑。
    fn encode(
        &self,
        ctx: &TenantContext,
        ttl_seconds: u64,
        token_type: &str,
        jti: Option<String>,
    ) -> Result<String, AuthError> {
        let exp = (now_epoch_seconds() + ttl_seconds) as usize;
        let claims = Claims {
            sub: ctx.user_id.clone(),
            tenant_id: ctx.tenant_id.clone(),
            roles: ctx.roles.clone(),
            permissions: ctx.permissions.clone(),
            exp,
            token_type: token_type.to_string(),
            jti,
        };
        jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|err| AuthError::Internal(err.to_string()))
    }

    /// 内部解码逻辑，校验 token 类型。
    fn decode(&self, token: &str, expected_type: &str) -> Result<TenantContext, AuthError> {
        let decoded = self.decode_claims(token)?;
        if decoded.token_type != expected_type {
            return Err(AuthError::TokenInvalid);
        }
        Ok(TenantContext::new(
            decoded.tenant_id,
            decoded.sub,
            decoded.roles,
            decoded.permissions,
            None,
        ))
    }
}

impl JwtManager {
    fn decode_claims(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let decoded = jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &validation,
        )
        .map_err(map_jwt_error)?;
        Ok(decoded.claims)
    }
}

/// 当前时间戳（秒）。
fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 将 jwt 库错误映射为业务错误。
fn map_jwt_error(err: jsonwebtoken::errors::Error) -> AuthError {
    match err.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
        _ => AuthError::TokenInvalid,
    }
}
