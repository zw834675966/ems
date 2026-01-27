//! 存储层错误类型
//!
//! 定义统一的存储错误类型，用于封装底层错误：
//! - SQL 执行错误
//! - 连接错误
//! - 数据一致性错误

#[derive(Debug)]
pub struct StorageError {
    kind: StorageErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageErrorKind {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    Other(String),
}

impl StorageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: StorageErrorKind::Other(message.into()),
        }
    }

    pub fn is_connection_error(&self) -> bool {
        match &self.kind {
            StorageErrorKind::Database(e) => matches!(
                e,
                sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed
            ),
            _ => false,
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        Self {
            kind: StorageErrorKind::Database(err),
        }
    }
}
