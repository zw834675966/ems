//! 数据模型
//!
//! 定义所有存储相关的数据模型和更新结构：
//! - 用户模型：UserRecord
//! - 项目模型：ProjectRecord, ProjectUpdate
//! - 网关模型：GatewayRecord, GatewayUpdate
//! - 设备模型：DeviceRecord, DeviceUpdate
//! - 点位模型：PointRecord, PointUpdate
//! - 点映射模型：PointMappingRecord, PointMappingUpdate

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

/// 网关记录。
#[derive(Debug, Clone)]
pub struct GatewayRecord {
    pub gateway_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub name: String,
    pub status: String,
}

/// 网关更新输入。
#[derive(Debug, Clone)]
pub struct GatewayUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
}

/// 设备记录。
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub device_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub gateway_id: String,
    pub name: String,
    pub model: Option<String>,
}

/// 设备更新输入。
#[derive(Debug, Clone)]
pub struct DeviceUpdate {
    pub name: Option<String>,
    pub model: Option<String>,
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
}

/// 点位映射更新输入。
#[derive(Debug, Clone)]
pub struct PointMappingUpdate {
    pub source_type: Option<String>,
    pub address: Option<String>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
}
