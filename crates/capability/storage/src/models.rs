//! 数据模型
//!
//! 定义所有存储相关的数据模型和更新结构：
//! - 用户模型：UserRecord
//! - 项目模型：ProjectRecord, ProjectUpdate
//! - 楼宇层级：AreaRecord, BuildingRecord, FloorRecord, RoomRecord
//! - 网关模型：GatewayRecord, GatewayUpdate（含协议配置）
//! - 设备模型：DeviceRecord, DeviceUpdate（含地址配置）
//! - 点位模型：PointRecord, PointUpdate
//! - 点映射模型：PointMappingRecord, PointMappingUpdate（含协议细节）
//! - 时序与实时模型：MeasurementRecord, RealtimeRecord

/// 用户记录（用于 M0 演示）。
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub tenant_id: String,
    pub user_id: String,
    pub username: String,
    pub password: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl UserRecord {
    /// 将用户记录转换为 TenantContext。
    pub fn to_tenant_context(&self) -> domain::TenantContext {
        domain::TenantContext::new(
            self.tenant_id.clone(),
            self.user_id.clone(),
            self.roles.clone(),
            self.permissions.clone(),
            None,
        )
    }
}

/// RBAC 用户（管理面用，避免返回密码字段）。
#[derive(Debug, Clone)]
pub struct RbacUserRecord {
    pub tenant_id: String,
    pub user_id: String,
    pub username: String,
    pub status: String,
    pub roles: Vec<String>,
}

/// RBAC 用户创建输入（管理面用）。
#[derive(Debug, Clone)]
pub struct RbacUserCreate {
    pub user_id: String,
    pub tenant_id: String,
    pub username: String,
    pub password: String,
    pub status: String,
    pub roles: Vec<String>,
}

/// RBAC 用户更新输入（管理面用）。
#[derive(Debug, Clone)]
pub struct RbacUserUpdate {
    pub password: Option<String>,
    pub status: Option<String>,
}

/// RBAC 角色（管理面用）。
#[derive(Debug, Clone)]
pub struct RbacRoleRecord {
    pub tenant_id: String,
    pub role_code: String,
    pub name: String,
    pub permissions: Vec<String>,
}

/// RBAC 角色创建输入（管理面用）。
#[derive(Debug, Clone)]
pub struct RbacRoleCreate {
    pub tenant_id: String,
    pub role_code: String,
    pub name: String,
    pub permissions: Vec<String>,
}

/// 权限码（管理面用）。
#[derive(Debug, Clone)]
pub struct PermissionRecord {
    pub permission_code: String,
    pub description: String,
}

/// 项目记录（用于租户归属校验）。
#[derive(Debug, Clone)]
pub struct ProjectRecord {
    pub project_id: String,
    pub tenant_id: String,
    pub name: String,
    pub timezone: String,
}

/// 项目更新输入。
#[derive(Debug, Clone)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub timezone: Option<String>,
}

// ============================================================================
// 楼宇层级模型（区域 → 楼宇 → 楼层 → 房间）
// ============================================================================

/// 区域记录
#[derive(Debug, Clone)]
pub struct AreaRecord {
    pub area_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
}

/// 区域更新输入
#[derive(Debug, Clone)]
pub struct AreaUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// 楼宇记录
#[derive(Debug, Clone)]
pub struct BuildingRecord {
    pub building_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub area_id: String,
    pub name: String,
    pub address: Option<String>,
}

/// 楼宇更新输入
#[derive(Debug, Clone)]
pub struct BuildingUpdate {
    pub name: Option<String>,
    pub address: Option<String>,
}

/// 楼层记录
#[derive(Debug, Clone)]
pub struct FloorRecord {
    pub floor_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub building_id: String,
    pub floor_number: i32,
    pub floor_name: Option<String>,
}

/// 楼层更新输入
#[derive(Debug, Clone)]
pub struct FloorUpdate {
    pub floor_number: Option<i32>,
    pub floor_name: Option<String>,
}

/// 房间记录
#[derive(Debug, Clone)]
pub struct RoomRecord {
    pub room_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub floor_id: String,
    pub room_number: String,
    pub room_name: Option<String>,
    pub room_type: Option<String>,
}

/// 房间更新输入
#[derive(Debug, Clone)]
pub struct RoomUpdate {
    pub room_number: Option<String>,
    pub room_name: Option<String>,
    pub room_type: Option<String>,
}

// ============================================================================
// 网关与设备模型（含协议配置）
// ============================================================================

/// 网关记录。
///
/// 网关支持多种协议类型：
/// - `mqtt`: MQTT 协议
/// - `modbus_tcp`: Modbus TCP 协议  
/// - `tcp_server`: TCP 服务器模式
/// - `tcp_client`: TCP 客户端模式
#[derive(Debug, Clone)]
pub struct GatewayRecord {
    pub gateway_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub name: String,
    pub status: String,
    /// 协议类型: mqtt | modbus_tcp | tcp_server | tcp_client
    pub protocol_type: String,
    /// 协议配置（JSON 格式）
    pub protocol_config: Option<String>,
}

/// 网关更新输入。
#[derive(Debug, Clone)]
pub struct GatewayUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub protocol_type: Option<String>,
    pub protocol_config: Option<String>,
}

/// 设备记录。
///
/// 设备可关联到房间，并根据网关协议类型配置地址。
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub device_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub gateway_id: String,
    pub name: String,
    pub model: Option<String>,
    /// 设备所在房间（可选）
    pub room_id: Option<String>,
    /// 协议地址配置（JSON 格式）
    pub address_config: Option<String>,
}

/// 设备更新输入。
#[derive(Debug, Clone)]
pub struct DeviceUpdate {
    pub name: Option<String>,
    pub model: Option<String>,
    pub room_id: Option<String>,
    pub address_config: Option<String>,
}

/// 点位记录。
#[derive(Debug, Clone)]
pub struct PointRecord {
    pub point_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub device_id: String,
    pub key: String,
    pub data_type: String,
    pub unit: Option<String>,
    /// 协议细节配置（JSON 格式）
    pub protocol_detail: Option<String>,
}

/// 点位更新输入。
#[derive(Debug, Clone)]
pub struct PointUpdate {
    pub key: Option<String>,
    pub data_type: Option<String>,
    pub unit: Option<String>,
    pub protocol_detail: Option<String>,
}

/// 点位映射记录。
///
/// 点位映射定义了从外部数据源到内部点位的映射关系。
/// `protocol_detail` 根据协议类型存储特定配置：
/// - Modbus: `{"functionCode": 3, "registerAddress": 100, "registerCount": 1, "dataType": "int16", "wordOrder": "ABCD"}`
/// - TCP (TLV): `{"tag": 1, "valueType": "uint16", "endian": "big_endian"}`
/// - MQTT: `{"json_path": "$.sensors.temperature", "data_type": "float"}`
#[derive(Debug, Clone)]
pub struct PointMappingRecord {
    pub source_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub point_id: String,
    pub source_type: String,
    pub address: String,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    /// 协议细节配置（JSON 格式）
    pub protocol_detail: Option<String>,
}

/// 点位映射更新输入。
#[derive(Debug, Clone)]
pub struct PointMappingUpdate {
    pub source_type: Option<String>,
    pub address: Option<String>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub protocol_detail: Option<String>,
}

/// 时序测点记录。
#[derive(Debug, Clone)]
pub struct MeasurementRecord {
    pub tenant_id: String,
    pub project_id: String,
    pub point_id: String,
    pub ts_ms: i64,
    pub value: String,
    pub quality: Option<String>,
}

/// 实时测点记录（last_value）。
#[derive(Debug, Clone)]
pub struct RealtimeRecord {
    pub tenant_id: String,
    pub project_id: String,
    pub point_id: String,
    pub ts_ms: i64,
    pub value: String,
    pub quality: Option<String>,
}

/// 控制命令记录。
#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub command_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub target: String,
    pub payload: String,
    pub status: String,
    pub issued_by: String,
    pub issued_at_ms: i64,
}

/// 控制命令回执记录。
#[derive(Debug, Clone)]
pub struct CommandReceiptRecord {
    pub receipt_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub command_id: String,
    pub ts_ms: i64,
    pub status: String,
    pub message: Option<String>,
}

/// 审计日志记录。
#[derive(Debug, Clone)]
pub struct AuditLogRecord {
    pub audit_id: String,
    pub tenant_id: String,
    pub project_id: Option<String>,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub result: String,
    pub detail: Option<String>,
    pub ts_ms: i64,
}

// ============================================================================
// 采集策略模型
// ============================================================================

/// 采集间隔时间单位
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntervalUnit {
    /// 毫秒
    Milliseconds,
    /// 秒
    Seconds,
    /// 分钟
    Minutes,
}

impl IntervalUnit {
    /// 从字符串解析
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ms" | "milliseconds" => Self::Milliseconds,
            "s" | "sec" | "seconds" => Self::Seconds,
            "min" | "minute" | "minutes" => Self::Minutes,
            _ => Self::Milliseconds,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Milliseconds => "ms",
            Self::Seconds => "s",
            Self::Minutes => "min",
        }
    }

    /// 转换采集间隔为毫秒
    pub fn to_millis(&self, value: i32) -> i64 {
        match self {
            Self::Milliseconds => value as i64,
            Self::Seconds => (value as i64) * 1000,
            Self::Minutes => (value as i64) * 60 * 1000,
        }
    }
}

impl std::fmt::Display for IntervalUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 采集策略记录。
///
/// 存储每个点位的自动采集配置，支持定时采集和时序数据库写入。
#[derive(Debug, Clone)]
pub struct CollectionStrategyRecord {
    /// 策略唯一标识
    pub strategy_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 项目 ID
    pub project_id: String,
    /// 点位 ID
    pub point_id: String,
    /// 是否启用采集
    pub enabled: bool,
    /// 采集间隔数值
    pub interval_value: i32,
    /// 采集间隔单位
    pub interval_unit: String,
    /// 是否写入时序数据库
    pub write_to_db: bool,
    /// 最后采集时间（毫秒时间戳）
    pub last_collected_at: Option<i64>,
    /// 最后采集值
    pub last_value: Option<String>,
    /// 最后错误信息
    pub last_error: Option<String>,
    /// 创建时间（毫秒时间戳）
    pub created_at: Option<i64>,
    /// 更新时间（毫秒时间戳）
    pub updated_at: Option<i64>,
}

impl CollectionStrategyRecord {
    /// 获取采集间隔（毫秒）
    pub fn interval_millis(&self) -> i64 {
        let unit = IntervalUnit::parse(&self.interval_unit);
        unit.to_millis(self.interval_value)
    }
}

/// 采集策略创建输入。
#[derive(Debug, Clone)]
pub struct CollectionStrategyCreate {
    pub strategy_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub point_id: String,
    pub enabled: bool,
    pub interval_value: i32,
    pub interval_unit: String,
    pub write_to_db: bool,
}

/// 采集策略更新输入。
#[derive(Debug, Clone, Default)]
pub struct CollectionStrategyUpdate {
    pub enabled: Option<bool>,
    pub interval_value: Option<i32>,
    pub interval_unit: Option<String>,
    pub write_to_db: Option<bool>,
    pub last_collected_at: Option<i64>,
    pub last_value: Option<String>,
    pub last_error: Option<String>,
}

// ============================================================================
// 系统日志模型（用于前端消息通知）
// ============================================================================

/// 日志分类
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogCategory {
    /// 操作日志（原"通知"）
    Operation,
    /// 错误日志（原"消息"）
    Error,
    /// 警告日志（原"待办"）
    Warning,
}

impl LogCategory {
    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "operation" => Some(Self::Operation),
            "error" => Some(Self::Error),
            "warning" => Some(Self::Warning),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Operation => "operation",
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl std::fmt::Display for LogCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 日志级别
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 系统日志记录
///
/// 用于前端消息通知展示的系统日志，包含操作日志、错误日志和警告日志。
#[derive(Debug, Clone)]
pub struct SystemLogRecord {
    /// 日志ID
    pub log_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 项目ID（可选，某些系统级日志不关联项目）
    pub project_id: Option<String>,
    /// 日志分类
    pub category: LogCategory,
    /// 日志级别
    pub level: LogLevel,
    /// 日志标题（简短描述）
    pub title: String,
    /// 详细消息
    pub message: String,
    /// 来源模块（如 'ems.ingest', 'ems.api.handlers'）
    pub source: Option<String>,
    /// 关联资源（如 'gateway:gw-123'）
    pub resource: Option<String>,
    /// 触发者（用户ID或系统）
    pub actor: Option<String>,
    /// 额外元数据（JSON格式）
    pub metadata: Option<String>,
    /// 是否已读
    pub is_read: bool,
    /// 创建时间（毫秒时间戳）
    pub created_at_ms: i64,
    /// 已读时间（毫秒时间戳）
    pub read_at_ms: Option<i64>,
}

/// 系统日志查询参数
#[derive(Debug, Clone, Default)]
pub struct SystemLogQuery {
    /// 按分类过滤
    pub category: Option<LogCategory>,
    /// 按级别过滤
    pub level: Option<LogLevel>,
    /// 仅查询未读
    pub unread_only: bool,
    /// 开始时间（毫秒时间戳）
    pub from_ms: Option<i64>,
    /// 结束时间（毫秒时间戳）
    pub to_ms: Option<i64>,
    /// 限制数量
    pub limit: i64,
}

/// 未读日志统计
#[derive(Debug, Clone, Default)]
pub struct UnreadStats {
    /// 未读操作日志数量
    pub operation: i64,
    /// 未读错误日志数量
    pub error: i64,
    /// 未读警告日志数量
    pub warning: i64,
    /// 未读总数
    pub total: i64,
}
