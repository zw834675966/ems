//! # PostgreSQL 存储实现模块
//!
//! 本模块提供所有存储接口的 PostgreSQL 实现，用于生产环境。
//!
//! ## 设计原则
//!
//! 1. **类型安全**：使用 sqlx 的类型安全查询，在编译时检查 SQL 语句
//! 2. **参数化查询**：所有 SQL 查询使用参数绑定，防止 SQL 注入攻击
//! 3. **多租户隔离**：所有查询都包含 `tenant_id` 过滤条件，确保租户数据隔离
//! 4. **项目作用域**：项目级资源操作包含 `project_id` 过滤条件
//! 5. **连接池管理**：使用连接池复用数据库连接，提高性能
//!
//! ## 包含的实现
//!
//! - **UserStore** (`user.rs`)：用户存储，支持登录验证和权限查询
//! - **ProjectStore** (`project.rs`)：项目存储，支持 CRUD 和租户归属校验
//! - **GatewayStore** (`gateway.rs`)：网关存储，支持项目级资源管理
//! - **DeviceStore** (`device.rs`)：设备存储，支持项目级资源管理
//! - **PointStore** (`point.rs`)：点位存储，支持项目级资源管理
//! - **PointMappingStore** (`point_mapping.rs`)：点位映射存储，支持项目级资源管理
//!
//! ## 数据库模式要求
//!
//! 本模块依赖以下数据库表：
//!
//! ### 核心表
//! - `users`：用户表（user_id, tenant_id, username, password_hash）
//! - `tenants`：租户表（tenant_id, name, status）
//! - `projects`：项目表（project_id, tenant_id, name, timezone）
//! - `roles`：角色表（role_code, name）
//! - `permissions`：权限表（permission_code, description）
//! - `user_roles`：用户角色关联表（user_id, role_code）
//! - `role_permissions`：角色权限关联表（role_code, permission_code）
//!
//! ### 资产表
//! - `gateways`：网关表（gateway_id, tenant_id, project_id, name, status, last_seen_at）
//! - `devices`：设备表（device_id, tenant_id, project_id, gateway_id, name, model）
//! - `points`：点位表（point_id, tenant_id, project_id, device_id, key, data_type, unit）
//! - `point_sources`：点位映射表（source_id, tenant_id, project_id, point_id, source_type, address, scale, offset_value）
//!
//! ## 性能优化
//!
//! ### 索引
//! - `idx_projects_tenant`：(tenant_id) 单列索引
//! - `idx_gateways_tenant_project`：(tenant_id, project_id) 复合索引
//! - `idx_devices_tenant_project`：(tenant_id, project_id) 复合索引
//! - `idx_points_tenant_project`：(tenant_id, project_id) 复合索引
//! - `idx_point_sources_tenant_project`：(tenant_id, project_id) 复合索引
//!
//! 这些索引确保：
//! - 租户级查询高效
//! - 项目级查询高效
//! - 避免 full table scan
//!
//! ### 查询优化
//! - 使用 `fetch_all` 批量查询，避免 N+1 问题
//! - 使用 `fetch_optional` 处理可能不存在的记录
//! - 使用 `RETURNING` 子句在更新/删除后返回数据，减少额外查询
//!
//! ## 连接池配置
//!
//! 默认配置（在 `connection.rs` 中）：
//! - 最大连接数：8
//! - 最小空闲连接：0
//! - 连接超时：30 秒（sqlx 默认）
//!
//! 可根据负载调整：
//! - 低负载（< 100 QPS）：4-8 连接
//! - 中等负载（100-1000 QPS）：8-16 连接
//! - 高负载（> 1000 QPS）：16-32 连接
//!
//! ## 安全考虑
//!
//! ### SQL 注入防护
//! - 所有查询使用参数绑定（`$1`, `$2` 等）
//! - 禁止字符串拼接构建 SQL
//! - sqlx 在编译时验证 SQL 语法
//!
//! ### 租户隔离
//! - 所有查询显式包含 `tenant_id` 条件
//! - 禁止跨租户查询
//! - 应用层验证（`TenantContext`）+ 数据层过滤双重保护
//!
//! ### 权限验证
//! - 用户密码使用哈希存储（生产环境应使用 bcrypt/argon2）
//! - 角色和权限通过关联表管理
//! - 支持细粒度权限控制
//!
//! ## 错误处理
//!
//! 所有存储操作返回 `Result<T, StorageError>`：
//!
//! - `StorageError`：封装底层错误，统一错误处理
//! - `sqlx::Error`：自动转换为 `StorageError`
//! - 返回 `Option<T>` 表示"可能不存在"（查询、更新、删除）
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use ems_storage::{PgUserStore, UserStore};
//! use domain::TenantContext;
//!
//! // 创建存储实例
//! let pool = connect_pool("postgresql://ems:admin123@localhost:5432/ems").await?;
//! let user_store = PgUserStore::new(pool);
//!
//! // 创建租户上下文
//! let ctx = TenantContext::new(
//!     "tenant-1".to_string(),
//!     "user-1".to_string(),
//!     vec![],
//!     vec![],
//!     None,
//! );
//!
//! // 查询用户（自动应用租户过滤）
//! let user = user_store.find_by_username(&ctx, "admin").await?;
//! ```
//!
//! ## 事务支持
//!
//! 当前实现不支持事务（MVP 范围）。
//!
//! 如需添加事务支持：
//! ```rust,ignore
//! let mut tx = pool.begin().await?;
//! // 执行多个操作
//! tx.commit().await?;
//! ```
//!
//! ## 未来扩展
//!
//! - **批量操作**：支持批量插入、批量更新
//! - **分页查询**：支持游标分页和偏移分页
//! - **复杂查询**：支持 JOIN、GROUP BY、聚合函数
//! - **全文搜索**：支持用户名、项目名的全文检索
//! - **数据归档**：支持历史数据的归档和清理

// 导出各个 PostgreSQL 存储实现
pub mod device;
pub mod gateway;
pub mod point;
pub mod point_mapping;
pub mod project;
pub mod user;

// 导出到 crate 根目录，方便外部引用
pub use device::*;
pub use gateway::*;
pub use point::*;
pub use point_mapping::*;
pub use project::*;
pub use user::*;
