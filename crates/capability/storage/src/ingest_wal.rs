//! 采集 WAL（Write-Ahead Log）存储
//!
//! 提供原始事件的持久化缓存能力，防止进程崩溃时数据丢失。
//! 在事件成功写入 PostgreSQL 后再从 WAL 中移除。

use crate::error::StorageError;
use async_trait::async_trait;
use domain::RawEvent;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

/// WAL 条目，包含唯一 ID 和原始事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// WAL 条目唯一 ID（UUID）
    pub wal_id: String,
    /// 原始采集事件
    pub event: RawEvent,
}

/// 采集 WAL 存储接口
///
/// 提供原始事件的持久化缓存能力：
/// - `push_event`: 事件入队（写入 WAL）
/// - `ack_event`: 事件确认（从 WAL 移除）
/// - `list_pending`: 列出待处理事件（用于崩溃恢复）
#[async_trait]
pub trait IngestWalStore: Send + Sync {
    /// 将原始事件推入 WAL，返回 WAL 条目 ID。
    async fn push_event(&self, event: &RawEvent) -> Result<String, StorageError>;

    /// 确认事件已成功处理，从 WAL 中移除。
    async fn ack_event(&self, wal_id: &str) -> Result<bool, StorageError>;

    /// 列出所有待处理的 WAL 条目（用于崩溃恢复）。
    async fn list_pending(&self) -> Result<Vec<WalEntry>, StorageError>;

    /// 获取待处理事件数量。
    async fn pending_count(&self) -> Result<usize, StorageError>;
}

// ============================================================================
// Redis 实现
// ============================================================================

const WAL_KEY: &str = "ems:ingest:wal";
const WAL_PROCESSING_KEY: &str = "ems:ingest:wal:processing";

/// Redis 采集 WAL 存储实现
///
/// 使用 Redis Hash 存储待处理事件：
/// - 主 Hash: `ems:ingest:wal` - 存储所有待处理条目
/// - 处理中 Hash: `ems:ingest:wal:processing` - 正在处理的条目（可选）
pub struct RedisIngestWalStore {
    client: redis::Client,
}

impl RedisIngestWalStore {
    /// 创建 Redis WAL 存储实例。
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    /// 从 Redis URL 连接创建 WAL 存储。
    pub fn connect(redis_url: &str) -> Result<Self, StorageError> {
        let client =
            redis::Client::open(redis_url).map_err(|err| StorageError::new(err.to_string()))?;
        Ok(Self::new(client))
    }
}

#[async_trait]
impl IngestWalStore for RedisIngestWalStore {
    async fn push_event(&self, event: &RawEvent) -> Result<String, StorageError> {
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        // 生成唯一 WAL ID
        let wal_id = uuid::Uuid::new_v4().to_string();

        // 序列化事件
        let entry = WalEntry {
            wal_id: wal_id.clone(),
            event: event.clone(),
        };
        let data =
            serde_json::to_string(&entry).map_err(|err| StorageError::new(err.to_string()))?;

        // 写入 Redis Hash
        connection
            .hset::<_, _, _, ()>(WAL_KEY, &wal_id, data)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        Ok(wal_id)
    }

    async fn ack_event(&self, wal_id: &str) -> Result<bool, StorageError> {
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        // 从 WAL 中删除
        let removed: i64 = connection
            .hdel(WAL_KEY, wal_id)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        // 同时从处理中 Hash 删除（如果存在）
        let _: i64 = connection
            .hdel(WAL_PROCESSING_KEY, wal_id)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        Ok(removed > 0)
    }

    async fn list_pending(&self) -> Result<Vec<WalEntry>, StorageError> {
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        // 获取所有 WAL 条目
        let entries: std::collections::HashMap<String, String> = connection
            .hgetall(WAL_KEY)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        let mut result = Vec::with_capacity(entries.len());
        for (_wal_id, data) in entries {
            match serde_json::from_str::<WalEntry>(&data) {
                Ok(entry) => result.push(entry),
                Err(err) => {
                    tracing::warn!(error = %err, "failed to parse WAL entry, skipping");
                }
            }
        }

        Ok(result)
    }

    async fn pending_count(&self) -> Result<usize, StorageError> {
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        let count: usize = connection
            .hlen(WAL_KEY)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        Ok(count)
    }
}

// ============================================================================
// 内存实现（用于测试）
// ============================================================================

/// 内存 WAL 存储实现（用于测试）
pub struct InMemoryIngestWalStore {
    entries: std::sync::RwLock<std::collections::HashMap<String, WalEntry>>,
}

impl InMemoryIngestWalStore {
    pub fn new() -> Self {
        Self {
            entries: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryIngestWalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IngestWalStore for InMemoryIngestWalStore {
    async fn push_event(&self, event: &RawEvent) -> Result<String, StorageError> {
        let wal_id = uuid::Uuid::new_v4().to_string();
        let entry = WalEntry {
            wal_id: wal_id.clone(),
            event: event.clone(),
        };

        let mut entries = self
            .entries
            .write()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        entries.insert(wal_id.clone(), entry);

        Ok(wal_id)
    }

    async fn ack_event(&self, wal_id: &str) -> Result<bool, StorageError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        Ok(entries.remove(wal_id).is_some())
    }

    async fn list_pending(&self) -> Result<Vec<WalEntry>, StorageError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        Ok(entries.values().cloned().collect())
    }

    async fn pending_count(&self) -> Result<usize, StorageError> {
        let entries = self
            .entries
            .read()
            .map_err(|_| StorageError::new("lock poisoned"))?;
        Ok(entries.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_event() -> RawEvent {
        RawEvent {
            tenant_id: "test-tenant".to_string(),
            project_id: "test-project".to_string(),
            source_id: "test-source".to_string(),
            address: "test/address".to_string(),
            payload: vec![1, 2, 3, 4],
            received_at_ms: 1234567890,
        }
    }

    #[tokio::test]
    async fn test_push_and_ack_event() {
        let store = InMemoryIngestWalStore::new();
        let event = make_test_event();

        // Push
        let wal_id = store.push_event(&event).await.unwrap();
        assert!(!wal_id.is_empty());

        // Verify pending
        let pending = store.list_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].wal_id, wal_id);

        // Ack
        let acked = store.ack_event(&wal_id).await.unwrap();
        assert!(acked);

        // Verify empty
        let pending = store.list_pending().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_pending_count() {
        let store = InMemoryIngestWalStore::new();
        let event = make_test_event();

        assert_eq!(store.pending_count().await.unwrap(), 0);

        let id1 = store.push_event(&event).await.unwrap();
        assert_eq!(store.pending_count().await.unwrap(), 1);

        let _id2 = store.push_event(&event).await.unwrap();
        assert_eq!(store.pending_count().await.unwrap(), 2);

        store.ack_event(&id1).await.unwrap();
        assert_eq!(store.pending_count().await.unwrap(), 1);
    }
}
