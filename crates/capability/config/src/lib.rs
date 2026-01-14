//! 应用运行配置加载。

use std::env;

/// 配置加载错误。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required env: {0}")]
    Missing(String),
    #[error("invalid value for {0}: {1}")]
    Invalid(String, String),
}

/// 应用运行配置。
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub http_addr: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_ttl_seconds: u64,
    pub jwt_refresh_ttl_seconds: u64,
}

impl AppConfig {
    /// 从环境变量读取配置。
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("EMS_DATABASE_URL")
            .map_err(|_| ConfigError::Missing("EMS_DATABASE_URL".to_string()))?;
        let jwt_secret = env::var("EMS_JWT_SECRET")
            .map_err(|_| ConfigError::Missing("EMS_JWT_SECRET".to_string()))?;
        let jwt_access_ttl_seconds = read_u64("EMS_JWT_ACCESS_TTL_SECONDS")?;
        let jwt_refresh_ttl_seconds = read_u64("EMS_JWT_REFRESH_TTL_SECONDS")?;
        let http_addr = env::var("EMS_HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

        Ok(Self {
            http_addr,
            database_url,
            jwt_secret,
            jwt_access_ttl_seconds,
            jwt_refresh_ttl_seconds,
        })
    }
}

/// 读取 u64 类型环境变量。
fn read_u64(key: &str) -> Result<u64, ConfigError> {
    let value = env::var(key).map_err(|_| ConfigError::Missing(key.to_string()))?;
    value
        .parse::<u64>()
        .map_err(|_| ConfigError::Invalid(key.to_string(), value))
}
