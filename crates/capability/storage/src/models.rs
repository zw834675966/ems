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
}

/// 点位更新输入。
#[derive(Debug, Clone)]
pub struct PointUpdate {
    pub key: Option<String>,
    pub data_type: Option<String>,
    pub unit: Option<String>,
}

/// 点位映射记录。
///
/// 点位映射定义了从外部数据源到内部点位的映射关系。
/// `protocol_detail` 根据协议类型存储特定配置：
/// - Modbus: `{"function_code": 3, "register_address": 100, "register_count": 1, "data_type": "int16"}`
/// - TCP: `{"byte_offset": 2, "byte_length": 2, "data_type": "uint16", "endian": "big"}`
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
