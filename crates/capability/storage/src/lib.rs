//! # EMS Storage 模块
//!
//! 本模块提供统一的数据存储抽象层，支持多种存储后端实现。
//!
//! ## 架构设计
//!
//! 该模块采用分层架构，遵循以下原则：
//!
//! 1. **接口抽象层** (`traits.rs`)：定义所有资源存储的异步 Trait 接口
//! 2. **数据模型层** (`models.rs`)：定义存储相关的数据结构
//! 3. **错误处理层** (`error.rs`)：统一的存储错误类型
//! 4. **验证辅助层** (`validation.rs`)：多租户和项目作用域验证
//! 5. **连接管理层** (`connection.rs`)：数据库连接池管理
//! 6. **实现层**：
//!    - `in_memory/`：内存存储实现（用于测试和演示）
//!    - `postgres/`：PostgreSQL 存储实现（生产环境使用）
//!
//! ## 核心特性
//!
//! - **多租户隔离**：所有存储接口都显式接收 `TenantContext`，确保租户数据隔离
//! - **项目作用域**：项目级资源操作自动验证项目归属权限
//! - **类型安全**：使用 Rust 的类型系统和 sqlx 的编译时 SQL 检查
//! - **异步支持**：基于 Tokio 的异步 I/O，支持高并发场景
//! - **可扩展性**：通过 Trait 接口支持多种存储后端
//!
//! ## 模块说明
//!
//! ### 核心模块
//!
//! - [`models`]：数据模型定义（用户、项目、网关、设备、点位、点位映射）
//! - [`traits`]：存储接口定义（CRUD 操作 + 归属校验）
//! - [`error`]：存储错误类型定义
//! - [`validation`]：租户和项目作用域验证函数
//! - [`connection`]：PostgreSQL 连接池管理
//!
//! ### 存储实现
//!
//! - [`in_memory`]：内存存储实现
//!   - 使用 `RwLock<HashMap>` 提供线程安全的内存存储
//!   - 适用于单元测试、集成测试和 M0 演示
//!   - 内置默认 admin 账户和默认项目
//!
//! - [`postgres`]：PostgreSQL 存储实现
//!   - 使用 sqlx 提供类型安全的数据库访问
//!   - 支持连接池管理（最大连接数 8）
//!   - 所有 SQL 查询使用参数化，防止 SQL 注入
//!   - 生产环境推荐使用
//!
//! ## 使用示例
//!
//! ### 使用 PostgreSQL 存储（生产环境）
//!
//! ```rust,ignore
//! use ems_storage::{PgUserStore, UserStore, connect_pool};
//! use domain::TenantContext;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 建立连接池
//!     let pool = connect_pool("postgresql://ems:admin123@localhost:5432/ems").await?;
//!
//!     // 创建用户存储
//!     let user_store = PgUserStore::new(pool);
//!
//!     // 创建租户上下文
//!     let ctx = TenantContext::new(
//!         "tenant-1".to_string(),
//!         "user-1".to_string(),
//!         vec!["admin".to_string()],
//!         vec![],
//!         None,
//!     );
//!
//!     // 查询用户
//!     let user = user_store.find_by_username(&ctx, "admin").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 使用内存存储（测试环境）
//!
//! ```rust,ignore
//! use ems_storage::{InMemoryUserStore, UserStore};
//! use domain::TenantContext;
//!
//! // 创建带默认 admin 的存储
//! let user_store = InMemoryUserStore::with_default_admin();
//!
//! // 查询用户
//! let ctx = TenantContext::default();
//! let user = user_store.find_by_username(&ctx, "admin").await?;
//! ```
//!
//! ## 多租户安全
//!
//! 所有存储操作都强制通过 `TenantContext` 进行租户隔离：
//!
//! - **租户 ID 验证**：所有查询自动添加 `tenant_id` 过滤条件
//! - **项目归属校验**：项目级资源操作前验证项目归属当前租户
//! - **禁止越权访问**：接口设计上防止跨租户数据访问
//!
//! ## 数据模型
//!
//! 本模块定义以下数据模型：
//!
//! - **UserRecord**：用户记录（tenant_id, user_id, username, password_hash, roles, permissions）
//! - **ProjectRecord**：项目记录（project_id, tenant_id, name, timezone）
//! - **GatewayRecord**：网关记录（gateway_id, tenant_id, project_id, name, status）
//! - **DeviceRecord**：设备记录（device_id, tenant_id, project_id, gateway_id, name, model）
//! - **PointRecord**：点位记录（point_id, tenant_id, project_id, device_id, key, data_type, unit）
//! - **PointMappingRecord**：点位映射记录（source_id, tenant_id, project_id, point_id, source_type, address, scale, offset）
//!
//! ## 设计约束
//!
//! - **禁止直接 SQL**：Handler 层禁止直接写 SQL，统一通过 storage 层
//! - **显式上下文**：所有数据访问方法必须显式接收 `TenantContext`
//! - **项目作用域**：项目内资源操作需保证 `TenantContext.project_scope` 与 `project_id` 一致
//!
//! ## 性能考虑
//!
//! - **连接池**：PostgreSQL 连接池最大连接数为 8，可根据负载调整
//! - **索引优化**：数据库表已建立 `(tenant_id, project_id)` 复合索引
//! - **批量查询**：列表查询使用 `fetch_all`，避免 N+1 查询问题
//! - **参数化查询**：所有 SQL 使用参数绑定，防止 SQL 注入且支持查询计划缓存
//!
//! ## 测试覆盖
//!
//! - 单元测试：内存实现的 CRUD 操作
//! - 集成测试：PostgreSQL 实现的完整功能
//! - 租户隔离测试：验证多租户隔离机制
//! - 项目归属测试：验证项目作用域校验逻辑
//!
//! ## 未来扩展
//!
//! - **Redis 集成**：添加 Redis 存储实现用于实时数据缓存
//! - **TimescaleDB**：添加时序数据存储支持
//! - **读写分离**：支持主从数据库配置
//! - **连接池调优**：支持动态调整连接池大小

// 模块导出：将子模块的内容导出到 crate 根目录
pub mod connection;
pub mod error;
pub mod in_memory;
pub mod models;
pub mod online;
pub mod postgres;
pub mod redis;
pub mod traits;
pub mod validation;

// 导出常用类型到 crate 根目录，方便外部引用
pub use connection::*;
pub use error::*;
pub use models::*;
pub use online::*;
pub use redis::RedisRealtimeStore;
pub use redis::RedisOnlineStore;
pub use traits::*;
pub use validation::*;

// 导出内存存储实现类型
pub use in_memory::{
    InMemoryAuditLogStore, InMemoryCommandReceiptStore, InMemoryCommandStore, InMemoryDeviceStore,
    InMemoryGatewayStore, InMemoryMeasurementStore, InMemoryPointMappingStore, InMemoryPointStore,
    InMemoryOnlineStore, InMemoryProjectStore, InMemoryRealtimeStore, InMemoryUserStore,
};

// 导出 PostgreSQL 存储实现类型
pub use postgres::{
    PgAuditLogStore, PgCommandReceiptStore, PgCommandStore, PgDeviceStore, PgGatewayStore,
    PgMeasurementStore, PgPointMappingStore, PgPointStore, PgProjectStore, PgUserStore,
};
