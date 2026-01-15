//! 存储层错误类型
//!
//! 定义统一的存储错误类型，用于封装底层错误：
//! - SQL 执行错误
//! - 连接错误
//! - 数据一致性错误

#[derive(Debug)]
pub struct StorageError {
    message: String,
}

impl StorageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StorageError {}

impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        Self::new(err.to_string())
    }
}
