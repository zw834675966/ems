//! 数据库连接管理
//!
//! 提供数据库连接池初始化功能：
//! - connect_pool：建立 Postgres 连接池
//!
//! 设计原则：
//! - 最大连接数限制为 8
//! - 使用 sqlx 提供的类型安全查询

use crate::error::StorageError;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// 建立 Postgres 连接池
///
/// 从数据库 URL 创建连接池，最大连接数限制为 8。
///
/// # 参数
/// - `database_url`：Postgres 连接字符串
///
/// # 返回
/// - `Result<PgPool, StorageError>`：连接池或错误
pub async fn connect_pool(database_url: &str) -> Result<PgPool, StorageError> {
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(database_url)
        .await?;
    Ok(pool)
}
