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
    pub redis_url: String,
    pub redis_last_value_ttl_seconds: Option<u64>,
    pub redis_online_ttl_seconds: u64,
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub mqtt_username: Option<String>,
    pub mqtt_password: Option<String>,
    pub mqtt_topic_prefix: String,
    pub mqtt_data_topic_prefix: String,
    pub mqtt_data_topic_has_source_id: bool,
    pub mqtt_command_topic_prefix: String,
    pub mqtt_command_topic_include_target: bool,
    pub mqtt_receipt_topic_prefix: String,
    pub mqtt_command_qos: u8,
    pub mqtt_receipt_qos: u8,
    pub ingest_enabled: bool,
    pub control_enabled: bool,
    pub control_dispatch_max_retries: u64,
    pub control_dispatch_backoff_ms: u64,
    pub control_receipt_timeout_seconds: u64,
    pub jwt_secret: String,
    pub jwt_access_ttl_seconds: u64,
    pub jwt_refresh_ttl_seconds: u64,
    pub require_timescale: bool,
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
        let redis_url = env::var("EMS_REDIS_URL")
            .unwrap_or_else(|_| "redis://default:admin123@localhost:6379".to_string());
        let redis_last_value_ttl_seconds =
            read_optional_u64("EMS_REDIS_LAST_VALUE_TTL_SECONDS")?.filter(|value| *value > 0);
        let redis_online_ttl_seconds = read_u64_with_default("EMS_REDIS_ONLINE_TTL_SECONDS", 60)?;
        let mqtt_host = env::var("EMS_MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let mqtt_port = read_u16_with_default("EMS_MQTT_PORT", 1883)?;
        let mqtt_username = read_optional("EMS_MQTT_USERNAME");
        let mqtt_password = read_optional("EMS_MQTT_PASSWORD");
        let mqtt_topic_prefix =
            env::var("EMS_MQTT_TOPIC_PREFIX").unwrap_or_else(|_| "ems".to_string());
        let mqtt_data_topic_prefix = env::var("EMS_MQTT_DATA_TOPIC_PREFIX").unwrap_or_else(|_| {
            format!("{}/data", mqtt_topic_prefix.trim_end_matches('/'))
        });
        let mqtt_data_topic_has_source_id =
            read_bool_with_default("EMS_MQTT_DATA_TOPIC_HAS_SOURCE_ID", false);
        let mqtt_command_topic_prefix = env::var("EMS_MQTT_COMMAND_TOPIC_PREFIX")
            .unwrap_or_else(|_| format!("{}/commands", mqtt_topic_prefix));
        let mqtt_command_topic_include_target =
            read_bool_with_default("EMS_MQTT_COMMAND_TOPIC_INCLUDE_TARGET", false);
        let mqtt_receipt_topic_prefix = env::var("EMS_MQTT_RECEIPT_TOPIC_PREFIX")
            .unwrap_or_else(|_| format!("{}/receipts", mqtt_topic_prefix));
        let mqtt_command_qos = read_u8_with_default("EMS_MQTT_COMMAND_QOS", 1)?;
        let mqtt_receipt_qos = read_u8_with_default("EMS_MQTT_RECEIPT_QOS", 1)?;
        let ingest_enabled = read_bool_with_default("EMS_INGEST", false);
        let control_enabled = read_bool_with_default("EMS_CONTROL", false);
        let control_dispatch_max_retries =
            read_u64_with_default("EMS_CONTROL_DISPATCH_MAX_RETRIES", 2)?;
        let control_dispatch_backoff_ms =
            read_u64_with_default("EMS_CONTROL_DISPATCH_BACKOFF_MS", 200)?;
        let control_receipt_timeout_seconds =
            read_u64_with_default("EMS_CONTROL_RECEIPT_TIMEOUT_SECONDS", 30)?;
        let require_timescale = read_bool_with_default("EMS_REQUIRE_TIMESCALE", false);

        Ok(Self {
            http_addr,
            database_url,
            redis_url,
            redis_last_value_ttl_seconds,
            redis_online_ttl_seconds,
            mqtt_host,
            mqtt_port,
            mqtt_username,
            mqtt_password,
            mqtt_topic_prefix,
            mqtt_data_topic_prefix,
            mqtt_data_topic_has_source_id,
            mqtt_command_topic_prefix,
            mqtt_command_topic_include_target,
            mqtt_receipt_topic_prefix,
            mqtt_command_qos,
            mqtt_receipt_qos,
            ingest_enabled,
            control_enabled,
            control_dispatch_max_retries,
            control_dispatch_backoff_ms,
            control_receipt_timeout_seconds,
            jwt_secret,
            jwt_access_ttl_seconds,
            jwt_refresh_ttl_seconds,
            require_timescale,
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

fn read_u16_with_default(key: &str, default: u16) -> Result<u16, ConfigError> {
    let value = match env::var(key) {
        Ok(value) => value,
        Err(_) => return Ok(default),
    };
    value
        .parse::<u16>()
        .map_err(|_| ConfigError::Invalid(key.to_string(), value))
}

fn read_u8_with_default(key: &str, default: u8) -> Result<u8, ConfigError> {
    let value = match env::var(key) {
        Ok(value) => value,
        Err(_) => return Ok(default),
    };
    value
        .parse::<u8>()
        .map_err(|_| ConfigError::Invalid(key.to_string(), value))
}

fn read_u64_with_default(key: &str, default: u64) -> Result<u64, ConfigError> {
    let value = match env::var(key) {
        Ok(value) => value,
        Err(_) => return Ok(default),
    };
    value
        .parse::<u64>()
        .map_err(|_| ConfigError::Invalid(key.to_string(), value))
}

fn read_optional(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

fn read_optional_u64(key: &str) -> Result<Option<u64>, ConfigError> {
    match env::var(key) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value
            .parse::<u64>()
            .map(Some)
            .map_err(|_| ConfigError::Invalid(key.to_string(), value)),
        Err(_) => Ok(None),
    }
}

fn read_bool_with_default(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(value) => matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "on"),
        Err(_) => default,
    }
}
