//! # EMS API 主入口
//!
//! 这是整个 EMS (Energy Management System - 能源管理系统) API 服务的主程序入口。
//! 本模块负责整个后端服务的初始化、配置加载和生命周期管理。
//!
//! ## 技术架构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        EMS API 服务架构                              │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────┐ │
//! │  │ HTTP 请求    │────▶│ Axum 路由器  │────▶│ 请求处理器 (handlers)│ │
//! │  └──────────────┘     └──────────────┘     └──────────────────────┘ │
//! │                              │                        │             │
//! │                              ▼                        ▼             │
//! │                       ┌──────────────┐        ┌──────────────┐      │
//! │                       │ 中间件层      │        │ 业务服务层    │      │
//! │                       │ - 认证校验    │        │ - AuthService│      │
//! │                       │ - 请求追踪    │        │ - CommandSvc │      │
//! │                       └──────────────┘        └──────────────┘      │
//! │                                                      │              │
//! │                                                      ▼              │
//! │  ┌──────────────────────────────────────────────────────────────┐  │
//! │  │                      存储层 (Storage Layer)                   │  │
//! │  ├──────────────────────────────────────────────────────────────┤  │
//! │  │  PostgreSQL 存储          │  Redis 存储      │  MQTT 消息     │  │
//! │  │  - 用户/项目/设备/测点    │  - 实时数据缓存   │  - 控制指令    │  │
//! │  │  - 历史测量数据          │  - 在线状态缓存   │  - 回执监听    │  │
//! │  │  - 控制指令/审计日志     │                  │                │  │
//! │  └──────────────────────────────────────────────────────────────┘  │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 主要功能模块
//!
//! 1. **Web Admin 启动管理**：
//!    - 根据环境变量 `EMS_WEB_ADMIN` 控制前端开发服务器的启动
//!    - 支持前后端联调开发模式
//!
//! 2. **应用状态初始化**：
//!    - 创建 PostgreSQL 数据库连接池
//!    - 初始化 JWT 认证服务
//!    - 创建各业务模块的存储层实例
//!
//! 3. **HTTP 服务器启动**：
//!    - 使用 Axum 框架构建 RESTful API
//!    - 支持请求追踪和审计日志
//!
//! ## 环境变量配置
//!
//! | 变量名 | 说明 | 可选值 | 默认值 |
//! |--------|------|--------|--------|
//! | `EMS_WEB_ADMIN` | 前端启动模式 | `off`/`on`/`only` | `off` |
//! | `DATABASE_URL` | PostgreSQL 连接字符串 | - | 必填 |
//! | `REDIS_URL` | Redis 连接字符串 | - | 必填 |
//! | `JWT_SECRET` | JWT 签名密钥 | - | 必填 |
//! | `HTTP_ADDR` | HTTP 监听地址 | - | `0.0.0.0:8080` |
//!
//! ### `EMS_WEB_ADMIN` 模式说明
//!
//! - `off`（默认）：不启动前端，仅运行后端 API 服务
//! - `on`：启动前端开发服务器（pnpm dev），同时运行后端 API
//! - `only`：仅启动前端开发服务器，不启动后端 API
//!
//! ## 启动流程
//!
//! ```text
//! 1. 加载 .env 环境变量 ──▶ 2. 读取应用配置 ──▶ 3. 初始化日志系统
//!                                                       │
//!    ┌───────────────────────────────────────────────────┘
//!    │
//!    ▼
//! 4. 处理前端启动 ──▶ 5. 建立数据库连接池 ──▶ 6. 初始化认证服务
//!                                                       │
//!    ┌───────────────────────────────────────────────────┘
//!    │
//!    ▼
//! 7. 初始化存储层 ──▶ 8. 构建路由器 ──▶ 9. 启动 HTTP 服务器
//! ```
//!
//! ## 依赖的内部 crate
//!
//! - [`ems_auth`]: 认证服务（JWT 令牌管理、用户认证）
//! - [`ems_config`]: 应用配置管理
//! - [`ems_storage`]: 存储层抽象和实现（PostgreSQL、Redis）
//! - [`ems_control`]: 设备控制服务（MQTT 指令分发）
//! - [`ems_telemetry`]: 遥测和日志系统

// ============================================================================
// 本地模块声明
// ============================================================================

/// HTTP 请求处理器模块
/// 包含所有 API 端点的具体处理逻辑（登录、项目管理、设备管理等）
mod handlers;

/// 数据采集模块
/// 负责从 MQTT 接收遥测数据并写入存储层
mod ingest;

/// HTTP 中间件模块
/// 包含请求上下文注入、认证校验等中间件
mod middleware;

/// 路由配置模块
/// 定义所有 API 路由及其对应的处理器
mod routes;

/// 工具函数模块
/// 包含通用的辅助函数和工具类
mod utils;

// ============================================================================
// 外部依赖导入
// ============================================================================

// Axum Web 框架 —— 高性能异步 HTTP 服务器框架
use axum::{Router, middleware as axum_middleware};

// 认证模块 —— JWT 令牌管理和用户认证服务
use ems_auth::{AuthService, JwtManager};

// 配置模块 —— 应用配置管理（从环境变量读取）
use ems_config::AppConfig;

// 控制模块 —— 设备控制指令发送和回执处理
use ems_control::{
    CommandService,            // 控制指令服务（封装指令创建、分发、重试逻辑）
    CommandServiceConfig,      // 控制服务配置（重试次数、超时等）
    MqttDispatcher,            // MQTT 指令分发器（通过 MQTT 发送控制指令）
    MqttDispatcherConfig,      // MQTT 分发器配置（连接信息、主题前缀等）
    MqttReceiptListenerConfig, // MQTT 回执监听器配置
    NoopDispatcher,            // 空操作分发器（用于禁用控制功能时）
    spawn_receipt_listener,    // 启动回执监听后台任务
};

// 存储模块 —— 数据持久化层实现
use ems_storage::{
    // PostgreSQL 存储实现
    PgAuditLogStore,       // 审计日志存储（记录用户操作）
    PgCommandReceiptStore, // 控制指令回执存储
    PgCommandStore,        // 控制指令存储
    PgDeviceStore,         // 设备信息存储
    PgGatewayStore,        // 网关信息存储
    PgMeasurementStore,    // 历史测量数据存储（时序数据）
    PgPointMappingStore,   // 测点映射存储（外部标识 → 内部 ID）
    PgPointStore,          // 测点定义存储
    PgProjectStore,        // 项目信息存储
    PgUserStore,           // 用户信息存储
    // Redis 存储实现
    RedisOnlineStore,   // 设备在线状态缓存
    RedisRealtimeStore, // 实时数据缓存（最新值）
    // 数据库连接工具
    connect_pool, // 创建 PostgreSQL 连接池
};

// 遥测模块 —— 日志和追踪系统初始化
use ems_telemetry::init_tracing;

// 标准库
use std::sync::Arc; // 原子引用计数（线程安全的共享所有权）
use std::{env, path::PathBuf}; // 环境变量访问和文件路径处理

// Tokio 异步运行时
use tokio::process::Command; // 异步子进程管理（用于启动前端）

// Tracing 日志宏
use tracing::{info, warn}; // 结构化日志输出

/// Web Admin 启动模式
///
/// 控制前端开发服务器 `web/admin` 的启动行为：
/// - `Off`：不启动前端，仅运行后端 API
/// - `On`：启动前端（pnpm dev），同时运行后端 API
/// - `Only`：仅启动前端，不启动后端 API
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WebAdminMode {
    Off,
    On,
    Only,
}

impl WebAdminMode {
    /// 从环境变量 `EMS_WEB_ADMIN` 读取启动模式
    ///
    /// 支持以下值：
    /// - `"1"`, `"true"`, `"on"` → `On`：前后端都启动
    /// - `"only"` → `Only`：仅启动前端
    /// - 其他值 → `Off`：不启动前端
    fn from_env() -> Self {
        match env::var("EMS_WEB_ADMIN")
            .unwrap_or_else(|_| "off".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "1" | "true" | "on" => Self::On,
            "only" => Self::Only,
            _ => Self::Off,
        }
    }
}

/// 启动前端开发服务器（web/admin）
///
/// 使用 `pnpm dev` 命令在 `web/admin` 目录启动前端开发服务器。
///
/// # 错误
///
/// - 如果 `web/admin` 目录不存在，返回 `NotFound` 错误
/// - 如果启动命令失败，返回相应的 I/O 错误
fn spawn_web_admin() -> Result<tokio::process::Child, std::io::Error> {
    // 获取当前 crate 的 manifest 目录
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // 定位 web/admin 目录（在 workspace 根目录下）
    let web_admin_dir = manifest_dir.join("../..").join("web/admin");
    if !web_admin_dir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("web/admin not found at {:?}", web_admin_dir),
        ));
    }
    // 启动 pnpm dev 进程
    Command::new("pnpm")
        .arg("dev")
        .current_dir(web_admin_dir)
        .spawn()
}

// ============================================================================
// 应用状态定义
// ============================================================================

/// 应用状态（AppState）
///
/// 这是整个 API 服务的核心状态容器，包含所有业务模块所需的服务和存储层实例。
/// 该结构体会被注入到每个 HTTP 请求处理器中，通过 Axum 的 `State` 提取器访问。
///
/// ## 设计原则
///
/// 1. **依赖注入**：所有存储层都通过 trait 对象（`dyn Trait`）传入，便于测试时替换为内存实现
/// 2. **线程安全**：所有字段都包装在 `Arc` 中，支持多线程并发访问
/// 3. **职责分离**：每个存储层只负责单一业务领域的数据访问
///
/// ## 存储层分类
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────────┐
/// │                        存储层架构                                    │
/// ├─────────────────────────────────────────────────────────────────────┤
/// │                                                                     │
/// │  ┌── 认证与权限 ──┐    ┌── 资产管理 ──┐    ┌── 数据采集 ──┐       │
/// │  │ auth           │    │ project_store │    │ measurement  │       │
/// │  │ rbac_store     │    │ gateway_store │    │ realtime     │       │
/// │  └────────────────┘    │ device_store  │    │ online       │       │
/// │                        │ point_store   │    │ point_mapping│       │
/// │                        └───────────────┘    └──────────────┘       │
/// │                                                                     │
/// │  ┌── 设备控制 ────────────────────────────────────────────┐        │
/// │  │ command_store / command_receipt_store / command_service │        │
/// │  └────────────────────────────────────────────────────────┘        │
/// │                                                                     │
/// │  ┌── 审计日志 ──┐                                                  │
/// │  │ audit_log    │                                                  │
/// │  └──────────────┘                                                  │
/// │                                                                     │
/// └─────────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Clone)]
struct AppState {
    // ========================================================================
    // 认证与权限模块
    // ========================================================================
    /// 认证服务
    ///
    /// 提供用户登录、JWT 令牌生成/验证、密码校验等认证功能。
    /// 内部封装了 `UserStore` 和 `JwtManager`。
    auth: Arc<AuthService>,

    /// 数据库连接池（可选）
    ///
    /// PostgreSQL 连接池，用于需要直接执行 SQL 查询的场景。
    /// 在测试环境中可能为 `None`（使用内存存储时）。
    db_pool: Option<sqlx::PgPool>,

    /// RBAC 权限存储
    ///
    /// 基于角色的访问控制存储，用于查询用户角色、权限等信息。
    /// 通常与 `PgUserStore` 共享实现。
    rbac_store: Arc<dyn ems_storage::RbacStore>,

    // ========================================================================
    // 资产管理模块
    // ========================================================================
    /// 项目存储
    ///
    /// 管理 EMS 项目的 CRUD 操作。
    /// 项目是资产层级的顶层，包含多个网关和设备。
    project_store: Arc<dyn ems_storage::ProjectStore>,

    /// 网关存储
    ///
    /// 管理网关设备的 CRUD 操作。
    /// 网关是连接边缘设备与云平台的桥梁，负责数据采集和指令下发。
    gateway_store: Arc<dyn ems_storage::GatewayStore>,

    /// 设备存储
    ///
    /// 管理物理设备的 CRUD 操作。
    /// 设备挂载在网关下，包含多个测点。
    device_store: Arc<dyn ems_storage::DeviceStore>,

    /// 测点存储
    ///
    /// 管理测点定义的 CRUD 操作。
    /// 测点是数据采集的最小单元，代表一个传感器或控制点。
    point_store: Arc<dyn ems_storage::PointStore>,

    /// 测点映射存储
    ///
    /// 管理外部标识到内部测点 ID 的映射关系。
    /// 用于数据上报时根据网关上报的标识查找对应的测点。
    point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,

    // ========================================================================
    // 数据采集模块
    // ========================================================================
    /// 历史测量数据存储
    ///
    /// 存储测点的历史时序数据，支持时间范围查询、聚合计算等。
    /// 后端使用 PostgreSQL + TimescaleDB 扩展实现高效的时序存储。
    measurement_store: Arc<dyn ems_storage::MeasurementStore>,

    /// 实时数据存储
    ///
    /// 存储测点的最新值（Last Value），用于实时监控场景。
    /// 后端使用 Redis 实现快速读写，数据带有 TTL 自动过期。
    realtime_store: Arc<dyn ems_storage::RealtimeStore>,

    /// 在线状态存储
    ///
    /// 存储设备/网关的在线状态，用于判断设备是否在线。
    /// 后端使用 Redis 实现，设备需周期性发送心跳刷新状态。
    online_store: Arc<dyn ems_storage::OnlineStore>,

    // ========================================================================
    // 设备控制模块
    // ========================================================================
    /// 控制指令存储
    ///
    /// 存储下发的控制指令记录，包括指令内容、状态、时间戳等。
    /// 支持指令查询、状态更新、历史追溯。
    command_store: Arc<dyn ems_storage::CommandStore>,

    /// 控制指令回执存储
    ///
    /// 存储设备返回的指令执行回执，用于确认指令是否成功执行。
    /// 注：当前代码中允许未使用（`#[allow(dead_code)]`）。
    #[allow(dead_code)]
    command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore>,

    /// 审计日志存储
    ///
    /// 存储用户操作的审计日志，包括登录、控制操作等。
    /// 用于安全审计和操作追溯。
    audit_log_store: Arc<dyn ems_storage::AuditLogStore>,

    /// 控制指令服务
    ///
    /// 封装控制指令的完整业务逻辑：
    /// - 创建控制指令记录
    /// - 通过 MQTT 分发器发送指令
    /// - 处理重试逻辑和超时
    /// - 记录审计日志
    command_service: Arc<CommandService>,
}

/// 主函数：EMS API 服务的入口点
///
/// 执行以下步骤初始化并启动服务：
///
/// 1. 加载环境变量（从 `.env` 文件）
/// 2. 读取应用配置（数据库 URL、JWT 配置、HTTP 监听地址等）
/// 3. 初始化 tracing 日志系统
/// 4. 根据 `EMS_WEB_ADMIN` 环境变量启动前端开发服务器（如果需要）
/// 5. 建立 PostgreSQL 数据库连接池
/// 6. 初始化认证服务（UserStore + JwtManager）
/// 7. 初始化各业务模块的 PostgreSQL 存储实现
/// 8. 创建应用状态并构建 Axum 路由器
/// 9. 添加请求上下文中间件（注入 request_id/trace_id）
/// 10. 绑定 TCP 监听器并启动 HTTP 服务器
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载 .env 文件中的环境变量（忽略错误）
    dotenvy::dotenv().ok();

    // 2. 从环境变量读取应用配置
    let config = AppConfig::from_env()?;

    // 3. 初始化 tracing 日志系统
    init_tracing();

    // 4. 处理 Web Admin 启动逻辑
    let web_admin_mode = WebAdminMode::from_env();
    let mut web_admin_child = None;
    if web_admin_mode != WebAdminMode::Off {
        // 尝试启动前端开发服务器
        match spawn_web_admin() {
            Ok(child) => {
                info!("web/admin started via pnpm dev");
                web_admin_child = Some(child);
            }
            Err(err) => {
                warn!("failed to start web/admin: {}", err);
                // 如果模式是 Only（仅前端），启动失败则直接退出
                if web_admin_mode == WebAdminMode::Only {
                    return Err(err.into());
                }
            }
        }
    }

    // 如果模式是 Only，等待前端进程退出后直接返回
    if web_admin_mode == WebAdminMode::Only {
        if let Some(mut child) = web_admin_child {
            let _ = child.wait().await?;
        }
        return Ok(());
    }

    // 如果前端已启动，在后台监控其退出状态
    if let Some(mut child) = web_admin_child {
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => info!("web/admin exited: {}", status),
                Err(err) => warn!("web/admin wait failed: {}", err),
            }
        });
    }

    // 5. 建立 PostgreSQL 数据库连接池
    let pool = connect_pool(&config.database_url).await?;

    if config.require_timescale {
        let has_timescaledb: Option<i32> =
            sqlx::query_scalar("select 1 from pg_extension where extname = 'timescaledb'")
                .fetch_optional(&pool)
                .await?;
        if has_timescaledb.is_none() {
            return Err("timescaledb extension is required (EMS_REQUIRE_TIMESCALE=on)".into());
        }
    }

    // 6. 初始化认证服务
    let user_store: Arc<PgUserStore> = Arc::new(PgUserStore::new(pool.clone()));
    let jwt = JwtManager::new(
        config.jwt_secret.clone(),
        config.jwt_access_ttl_seconds,
        config.jwt_refresh_ttl_seconds,
    );
    let auth: Arc<AuthService> = Arc::new(AuthService::new(user_store.clone(), jwt));
    let rbac_store: Arc<dyn ems_storage::RbacStore> = user_store.clone();

    // ========================================================================
    // 7. 初始化各业务模块的存储层实例
    // ========================================================================

    // --- 资产管理存储（PostgreSQL） ---
    // 项目存储：管理项目的增删改查
    let project_store: Arc<dyn ems_storage::ProjectStore> =
        Arc::new(PgProjectStore::new(pool.clone()));
    // 网关存储：管理网关设备
    let gateway_store: Arc<dyn ems_storage::GatewayStore> =
        Arc::new(PgGatewayStore::new(pool.clone()));
    // 设备存储：管理物理设备
    let device_store: Arc<dyn ems_storage::DeviceStore> =
        Arc::new(PgDeviceStore::new(pool.clone()));
    // 测点存储：管理测点定义
    let point_store: Arc<dyn ems_storage::PointStore> = Arc::new(PgPointStore::new(pool.clone()));
    // 测点映射存储：外部标识 → 内部 ID 的映射
    let point_mapping_store: Arc<dyn ems_storage::PointMappingStore> =
        Arc::new(PgPointMappingStore::new(pool.clone()));

    // --- 数据采集存储 ---
    // 历史测量数据存储（PostgreSQL + TimescaleDB）
    let measurement_store: Arc<dyn ems_storage::MeasurementStore> =
        Arc::new(PgMeasurementStore::new(pool.clone()));
    // 实时数据缓存（Redis）：存储测点的最新值
    let realtime_store: Arc<dyn ems_storage::RealtimeStore> =
        Arc::new(RedisRealtimeStore::connect_with_ttl(
            &config.redis_url,
            config.redis_last_value_ttl_seconds, // 最新值的过期时间（秒）
        )?);
    // 在线状态缓存（Redis）：存储设备在线状态
    let online_store: Arc<dyn ems_storage::OnlineStore> = Arc::new(RedisOnlineStore::connect(
        &config.redis_url,
        config.redis_online_ttl_seconds, // 在线状态的过期时间（秒）
    )?);

    // --- 设备控制存储（PostgreSQL） ---
    // 控制指令存储：记录下发的控制指令
    let command_store: Arc<dyn ems_storage::CommandStore> =
        Arc::new(PgCommandStore::new(pool.clone()));
    // 控制回执存储：记录设备返回的执行结果
    let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
        Arc::new(PgCommandReceiptStore::new(pool.clone()));
    // 审计日志存储：记录用户操作日志
    let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
        Arc::new(PgAuditLogStore::new(pool.clone()));

    // ========================================================================
    // 8. 初始化设备控制服务（MQTT 分发器）
    // ========================================================================
    //
    // 根据配置决定是否启用设备控制功能：
    // - 启用时：连接 MQTT Broker，通过 MQTT 发送控制指令
    // - 禁用时：使用空操作分发器（NoopDispatcher），不发送任何指令
    let (dispatcher, _dispatch_handle): (
        Arc<dyn ems_control::CommandDispatcher>,
        Option<tokio::task::JoinHandle<()>>,
    ) = if config.control_enabled {
        // 连接 MQTT Broker 并创建指令分发器
        let (mqtt_dispatcher, handle) = MqttDispatcher::connect(MqttDispatcherConfig {
            host: config.mqtt_host.clone(),         // MQTT 服务器地址
            port: config.mqtt_port,                 // MQTT 服务器端口
            username: config.mqtt_username.clone(), // MQTT 用户名
            password: config.mqtt_password.clone(), // MQTT 密码
            command_topic_prefix: config.mqtt_command_topic_prefix.clone(), // 指令主题前缀
            include_target_in_topic: config.mqtt_command_topic_include_target, // 是否在主题中包含目标
            qos: config.mqtt_command_qos,                                      // 消息服务质量等级
        })?;
        (Arc::new(mqtt_dispatcher), Some(handle))
    } else {
        // 控制功能禁用，使用空操作分发器
        (Arc::new(NoopDispatcher::default()), None)
    };

    // 创建控制指令服务（封装指令创建、分发、重试逻辑）
    let command_service = Arc::new(CommandService::new_with_config(
        command_store.clone(),
        audit_log_store.clone(),
        dispatcher.clone(),
        CommandServiceConfig {
            dispatch_max_retries: config.control_dispatch_max_retries, // 最大重试次数
            dispatch_backoff_ms: config.control_dispatch_backoff_ms,   // 重试退避时间（毫秒）
            receipt_timeout_ms: config.control_receipt_timeout_seconds.saturating_mul(1000), // 回执超时（毫秒）
        },
    ));

    // 启动 MQTT 回执监听器（如果控制功能启用）
    // 回执监听器会订阅回执主题，接收设备执行结果并更新指令状态
    let _receipt_handle = if config.control_enabled {
        Some(spawn_receipt_listener(
            MqttReceiptListenerConfig {
                host: config.mqtt_host.clone(),
                port: config.mqtt_port,
                username: config.mqtt_username.clone(),
                password: config.mqtt_password.clone(),
                receipt_topic_prefix: config.mqtt_receipt_topic_prefix.clone(), // 回执主题前缀
                qos: config.mqtt_receipt_qos,
            },
            command_store.clone(),
            command_receipt_store.clone(),
            audit_log_store.clone(),
        ))
    } else {
        None
    };

    // ========================================================================
    // 9. 启动数据采集服务（MQTT 遥测数据接收）
    // ========================================================================
    //
    // 数据采集服务订阅 MQTT 遥测主题，接收网关上报的测点数据：
    // 1. 根据测点映射查找内部测点 ID
    // 2. 将数据写入历史存储（PostgreSQL）
    // 3. 更新实时缓存（Redis 最新值）
    // 4. 更新设备在线状态
    let _ingest_handle = ingest::spawn_ingest(
        &config,
        point_mapping_store.clone(),
        point_store.clone(),
        device_store.clone(),
        measurement_store.clone(),
        realtime_store.clone(),
        online_store.clone(),
    );

    // ========================================================================
    // 10. 创建应用状态（AppState）
    // ========================================================================
    //
    // 将所有服务和存储层实例打包到 AppState 中，
    // 通过 Axum 的 `with_state()` 方法注入到路由器，
    // 使得每个请求处理器都可以访问这些共享资源。
    let state = AppState {
        auth,
        db_pool: Some(pool.clone()),
        rbac_store,
        project_store,
        gateway_store,
        device_store,
        point_store,
        point_mapping_store,
        measurement_store,
        realtime_store,
        online_store,
        command_store,
        command_receipt_store,
        audit_log_store,
        command_service,
    };

    // ========================================================================
    // 11. 构建 Axum 路由器
    // ========================================================================
    //
    // 路由器配置说明：
    // - `routes::create_api_router()`: 创建包含所有 API 端点的路由器
    // - `.merge(api.clone())`: 在根路径 `/` 下挂载 API（向后兼容）
    // - `.nest("/api", api)`: 在 `/api` 前缀下也挂载 API（推荐前缀）
    // - `.with_state(state)`: 注入应用状态
    // - `.layer(...)`: 添加请求上下文中间件（注入 request_id/trace_id）
    let api = routes::create_api_router();
    let app = Router::new()
        .merge(api.clone()) // 在根路径挂载 API
        .nest("/api", api) // 在 /api 前缀下也挂载 API
        .with_state(state) // 注入应用状态
        .layer(axum_middleware::from_fn(middleware::request_context)); // 添加请求追踪中间件

    // ========================================================================
    // 12. 绑定 TCP 监听器并启动 HTTP 服务器
    // ========================================================================
    //
    // 使用 Tokio 的异步 TCP 监听器绑定配置的地址，
    // 然后使用 Axum 的 `serve` 函数启动 HTTP 服务器。
    // 服务器会一直运行直到进程被终止。
    let listener = tokio::net::TcpListener::bind(&config.http_addr).await?;
    info!("🚀 EMS API 服务已启动，监听地址: {}", config.http_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

// ============================================================================
// 单元测试模块
// ============================================================================
//
// 本模块包含 EMS API 的单元测试，使用内存存储替代真实数据库，
// 以实现快速、隔离的测试执行。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::{get_realtime, list_measurements};
    use api_contract::{MeasurementsQuery, RealtimeQuery};
    use axum::extract::{Path, Query, State};
    use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
    use domain::{PointValue, PointValueData, TenantContext};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::Arc;

    // ========================================================================
    // 测试辅助函数
    // ========================================================================

    /// 构建测试用的 AppState（使用内存存储）
    ///
    /// 创建一个完整的 AppState 实例，但所有存储层都使用内存实现（InMemory*），
    /// 这样测试可以快速运行，不依赖外部数据库或 Redis。
    ///
    /// ## 默认数据
    ///
    /// - 用户：admin/admin123（通过 `InMemoryUserStore::with_default_admin()`）
    /// - 项目：默认项目（通过 `InMemoryProjectStore::with_default_project()`）
    ///
    /// ## 返回值
    ///
    /// 返回完全初始化的 AppState，可直接用于测试 HTTP 处理器。
    fn build_state() -> AppState {
        // --- 认证模块 ---
        // 创建内存用户存储，预置默认管理员账户
        let user_store: Arc<ems_storage::InMemoryUserStore> =
            Arc::new(ems_storage::InMemoryUserStore::with_default_admin());
        // 创建 JWT 管理器（测试用密钥和较长的 TTL）
        let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
        // 创建认证服务
        let auth: Arc<AuthService> = Arc::new(AuthService::new(user_store.clone(), jwt));
        // RBAC 存储复用用户存储
        let rbac_store: Arc<dyn ems_storage::RbacStore> = user_store.clone();

        // --- 资产管理存储（内存实现） ---
        let project_store: Arc<dyn ems_storage::ProjectStore> =
            Arc::new(ems_storage::InMemoryProjectStore::with_default_project());
        let gateway_store: Arc<dyn ems_storage::GatewayStore> =
            Arc::new(ems_storage::InMemoryGatewayStore::new());
        let device_store: Arc<dyn ems_storage::DeviceStore> =
            Arc::new(ems_storage::InMemoryDeviceStore::new());
        let point_store: Arc<dyn ems_storage::PointStore> =
            Arc::new(ems_storage::InMemoryPointStore::new());
        let point_mapping_store: Arc<dyn ems_storage::PointMappingStore> =
            Arc::new(ems_storage::InMemoryPointMappingStore::new());

        // --- 数据采集存储（内存实现） ---
        let measurement_store: Arc<dyn ems_storage::MeasurementStore> =
            Arc::new(ems_storage::InMemoryMeasurementStore::new());
        let realtime_store: Arc<dyn ems_storage::RealtimeStore> =
            Arc::new(ems_storage::InMemoryRealtimeStore::new());
        let online_store: Arc<dyn ems_storage::OnlineStore> =
            Arc::new(ems_storage::InMemoryOnlineStore::new());

        // --- 设备控制存储（内存实现） ---
        let command_store: Arc<dyn ems_storage::CommandStore> =
            Arc::new(ems_storage::InMemoryCommandStore::new());
        let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
            Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
        let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
            Arc::new(ems_storage::InMemoryAuditLogStore::new());

        // 使用空操作分发器（测试环境不发送实际 MQTT 消息）
        let dispatcher = Arc::new(ems_control::NoopDispatcher::default());
        let command_service = Arc::new(ems_control::CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            dispatcher,
        ));

        // 组装并返回 AppState
        AppState {
            auth,
            db_pool: None, // 测试环境不使用真实数据库连接池
            rbac_store,
            project_store,
            gateway_store,
            device_store,
            point_store,
            point_mapping_store,
            measurement_store,
            realtime_store,
            online_store,
            command_store,
            command_receipt_store,
            audit_log_store,
            command_service,
        }
    }

    /// 生成认证请求头（Bearer Token）
    ///
    /// 使用默认管理员账户（admin/admin123）登录，获取 JWT 令牌，
    /// 并返回包含 Authorization 头的 HeaderMap。
    ///
    /// ## 参数
    ///
    /// - `state`: 应用状态，用于执行登录操作
    ///
    /// ## 返回值
    ///
    /// 返回包含 `Authorization: Bearer <token>` 头的 HeaderMap。
    async fn auth_headers(state: &AppState) -> HeaderMap {
        // 使用默认管理员账户登录
        let (_, tokens) = state.auth.login("admin", "admin123").await.expect("login");
        let mut headers = HeaderMap::new();
        // 构造 Bearer Token 格式的 Authorization 头
        let value = format!("Bearer {}", tokens.access_token);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&value).expect("auth header"),
        );
        headers
    }

    /// 将 HTTP 响应体解析为 JSON
    ///
    /// ## 参数
    ///
    /// - `response`: Axum 的 HTTP 响应
    ///
    /// ## 返回值
    ///
    /// 返回解析后的 `serde_json::Value`。
    ///
    /// ## Panic
    ///
    /// 如果响应体读取或 JSON 解析失败，会 panic。
    async fn response_json(response: axum::response::Response) -> Value {
        let body = response.into_body();
        let bytes = body.collect().await.expect("collect body").to_bytes();
        serde_json::from_slice(&bytes).expect("json body")
    }

    // ========================================================================
    // 测试用例
    // ========================================================================

    /// 测试：获取实时数据（GET /projects/{project_id}/realtime）
    ///
    /// 验证实时数据 API 能够正确返回存储的测点最新值。
    ///
    /// ## 测试步骤
    ///
    /// 1. 创建测试 AppState（内存存储）
    /// 2. 向实时存储中写入一条测点数据
    /// 3. 调用 `get_realtime` 处理器
    /// 4. 验证响应状态码为 200 OK
    /// 5. 验证响应体包含正确的数据
    #[tokio::test]
    async fn realtime_returns_values() {
        // 准备测试环境
        let state = build_state();

        // 创建租户上下文（模拟已认证用户的请求上下文）
        let ctx = TenantContext::new(
            "tenant-1".to_string(),
            "user-1".to_string(),
            Vec::new(),
            Vec::new(),
            Some("project-1".to_string()),
        );

        // 创建测试数据：一条测点值
        let value = PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 1_700_000_000_000,          // 时间戳（毫秒）
            value: PointValueData::F64(12.34), // 浮点数值
            quality: None,                     // 质量标识（无）
        };

        // 写入实时存储
        state
            .realtime_store
            .upsert_last_value(&ctx, &value)
            .await
            .expect("upsert last value");

        // 调用处理器并验证响应
        let headers = auth_headers(&state).await;
        let response = get_realtime(
            State(state),
            Path(crate::handlers::realtime::ProjectPath {
                project_id: "project-1".to_string(),
            }),
            Query(RealtimeQuery { point_id: None }), // 查询所有测点
            headers,
        )
        .await;

        // 验证 HTTP 状态码
        assert_eq!(response.status(), StatusCode::OK);

        // 验证响应体内容
        let json = response_json(response).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"].as_array().map(|v| v.len()), Some(1));
    }

    /// 测试：获取历史测量数据（GET /projects/{project_id}/measurements）
    ///
    /// 验证历史数据 API 能够正确返回存储的测点历史值。
    ///
    /// ## 测试步骤
    ///
    /// 1. 创建测试 AppState（内存存储）
    /// 2. 向测量存储中写入一条历史数据
    /// 3. 调用 `list_measurements` 处理器
    /// 4. 验证响应状态码为 200 OK
    /// 5. 验证响应体包含正确的数据
    #[tokio::test]
    async fn measurements_returns_values() {
        // 准备测试环境
        let state = build_state();

        // 创建租户上下文
        let ctx = TenantContext::new(
            "tenant-1".to_string(),
            "user-1".to_string(),
            Vec::new(),
            Vec::new(),
            Some("project-1".to_string()),
        );

        // 创建测试数据：一条历史测量值
        let value = PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 1_700_000_000_100,          // 时间戳（毫秒）
            value: PointValueData::F64(23.45), // 浮点数值
            quality: Some("good".to_string()), // 质量标识：良好
        };

        // 写入历史存储
        state
            .measurement_store
            .write_measurement(&ctx, &value)
            .await
            .expect("write measurement");

        // 调用处理器并验证响应
        let headers = auth_headers(&state).await;
        let response = list_measurements(
            State(state),
            Path(crate::handlers::measurements::ProjectPath {
                project_id: "project-1".to_string(),
            }),
            Query(MeasurementsQuery {
                point_id: "point-1".to_string(), // 指定测点 ID
                from: None,                      // 起始时间（不限）
                to: None,                        // 结束时间（不限）
                limit: Some(100),                // 最多返回 100 条
                cursor_ts_ms: None,              // 游标（分页用）
                order: None,                     // 排序方式（默认）
                bucket_ms: None,                 // 聚合桶大小（不聚合）
                agg: None,                       // 聚合函数（不聚合）
            }),
            headers,
        )
        .await;

        // 验证 HTTP 状态码
        assert_eq!(response.status(), StatusCode::OK);

        // 验证响应体内容
        let json = response_json(response).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"].as_array().map(|v| v.len()), Some(1));
    }
}
