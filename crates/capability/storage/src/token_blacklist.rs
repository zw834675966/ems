//! Token 黑名单存储
//!
//! 提供 JWT Token 撤销能力，用于：
//! - 用户被禁用时立即失效其 Token
//! - 用户修改密码后使旧 Token 失效
//! - 用户主动登出时撤销 Token

use crate::error::StorageError;
use async_trait::async_trait;
use redis::AsyncCommands;

/// Token 黑名单存储接口
///
/// 提供 Token JTI（JWT ID）的撤销和查询能力：
/// - `add_to_blacklist`: 将 Token JTI 加入黑名单
/// - `is_blacklisted`: 检查 Token JTI 是否在黑名单中
#[async_trait]
pub trait TokenBlacklistStore: Send + Sync {
    /// 将 Token JTI 加入黑名单
    ///
    /// # 参数
    /// - `jti`: JWT ID
    /// - `ttl_seconds`: 黑名单有效期（应与 Token 剩余生命周期匹配）
    async fn add_to_blacklist(&self, jti: &str, ttl_seconds: u64) -> Result<(), StorageError>;

    /// 检查 Token JTI 是否在黑名单中
    async fn is_blacklisted(&self, jti: &str) -> Result<bool, StorageError>;

    /// 批量检查多个 JTI 是否在黑名单中
    async fn are_blacklisted(&self, jtis: &[String]) -> Result<Vec<bool>, StorageError>;
}

// ============================================================================
// Redis 实现
// ============================================================================

const BLACKLIST_KEY_PREFIX: &str = "ems:token:blacklist:";

/// Redis Token 黑名单存储实现
pub struct RedisTokenBlacklistStore {
    client: redis::Client,
}

impl RedisTokenBlacklistStore {
    /// 创建 Redis 黑名单存储实例
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    /// 从 Redis URL 连接创建黑名单存储
    pub fn connect(redis_url: &str) -> Result<Self, StorageError> {
        let client =
            redis::Client::open(redis_url).map_err(|err| StorageError::new(err.to_string()))?;
        Ok(Self::new(client))
    }

    fn key(jti: &str) -> String {
        format!("{}{}", BLACKLIST_KEY_PREFIX, jti)
    }
}

#[async_trait]
impl TokenBlacklistStore for RedisTokenBlacklistStore {
    async fn add_to_blacklist(&self, jti: &str, ttl_seconds: u64) -> Result<(), StorageError> {
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        let key = Self::key(jti);
        connection
            .set_ex::<_, _, ()>(key, "1", ttl_seconds)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        Ok(())
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool, StorageError> {
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        let key = Self::key(jti);
        let exists: bool = connection
            .exists(key)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        Ok(exists)
    }

    async fn are_blacklisted(&self, jtis: &[String]) -> Result<Vec<bool>, StorageError> {
        if jtis.is_empty() {
            return Ok(Vec::new());
        }

        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        let keys: Vec<String> = jtis.iter().map(|jti| Self::key(jti)).collect();

        // 使用 MGET，返回 Some/None 来判断是否存在
        let values: Vec<Option<String>> = connection
            .mget(&keys)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        Ok(values.into_iter().map(|v| v.is_some()).collect())
    }
}

// ============================================================================
// 内存实现（用于测试）
// ============================================================================

/// 内存 Token 黑名单存储实现（用于测试）
pub struct InMemoryTokenBlacklistStore {
    entries: std::sync::RwLock<std::collections::HashSet<String>>,
}

impl InMemoryTokenBlacklistStore {
    pub fn new() -> Self {
        Self {
            entries: std::sync::RwLock::new(std::collections::HashSet::new()),
        }
    }
}

impl Default for InMemoryTokenBlacklistStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TokenBlacklistStore for InMemoryTokenBlacklistStore {
    async fn add_to_blacklist(&self, jti: &str, _ttl_seconds: u64) -> Result<(), StorageError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        entries.insert(jti.to_string());
        Ok(())
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool, StorageError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        Ok(entries.contains(jti))
    }

    async fn are_blacklisted(&self, jtis: &[String]) -> Result<Vec<bool>, StorageError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        Ok(jtis.iter().map(|jti| entries.contains(jti)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_check_blacklist() {
        let store = InMemoryTokenBlacklistStore::new();
        let jti = "test-jti-123";

        // Initially not blacklisted
        assert!(!store.is_blacklisted(jti).await.unwrap());

        // Add to blacklist
        store.add_to_blacklist(jti, 3600).await.unwrap();

        // Now should be blacklisted
        assert!(store.is_blacklisted(jti).await.unwrap());
    }

    #[tokio::test]
    async fn test_batch_check() {
        let store = InMemoryTokenBlacklistStore::new();

        store.add_to_blacklist("jti-1", 3600).await.unwrap();
        store.add_to_blacklist("jti-3", 3600).await.unwrap();

        let jtis = vec![
            "jti-1".to_string(),
            "jti-2".to_string(),
            "jti-3".to_string(),
        ];

        let results = store.are_blacklisted(&jtis).await.unwrap();
        assert_eq!(results, vec![true, false, true]);
    }
}
