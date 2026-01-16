//! 存储接口 Trait 定义
//!
//! 定义所有资源存储的异步接口：
//! - UserStore：用户存储
//! - ProjectStore：项目存储
//! - AreaStore：区域存储
//! - BuildingStore：楼宇存储
//! - FloorStore：楼层存储
//! - RoomStore：房间存储
//! - GatewayStore：网关存储
//! - DeviceStore：设备存储
//! - PointStore：点存储
//! - PointMappingStore：点映射存储
//!
//! 设计原则：
//! - 所有接口显式接收 TenantContext
//! - 所有接口返回 StorageError
//! - 使用 async_trait 支持动态分发

use crate::error::StorageError;
use crate::models::{
    AreaRecord, AreaUpdate, AuditLogRecord, BuildingRecord, BuildingUpdate, CommandReceiptRecord,
    CommandRecord, DeviceRecord, DeviceUpdate, FloorRecord, FloorUpdate, GatewayRecord,
    GatewayUpdate, MeasurementRecord, PermissionRecord, PointMappingRecord, PointMappingUpdate,
    PointRecord, PointUpdate, ProjectRecord, ProjectUpdate, RbacRoleCreate, RbacRoleRecord,
    RbacUserCreate, RbacUserRecord, RbacUserUpdate, RealtimeRecord, RoomRecord, RoomUpdate,
    UserRecord,
};
use async_trait::async_trait;
use domain::{PointValue, TenantContext};

/// 用户存储接口
///
/// 提供用户查询功能（禁止在 handler 中直接连 SQL）。
#[async_trait]
pub trait UserStore: Send + Sync {
    /// 根据用户名查找用户
    async fn find_by_username(
        &self,
        ctx: &TenantContext,
        username: &str,
    ) -> Result<Option<UserRecord>, StorageError>;

    /// 更新用户口令哈希（用于登录时迁移旧口令存储，或管理面修改口令）。
    async fn update_password_hash(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        password_hash: &str,
    ) -> Result<bool, StorageError>;

    /// 获取当前有效 refresh token 的 jti（用于 refresh token rotation / 撤销）。
    async fn get_refresh_jti(
        &self,
        ctx: &TenantContext,
        user_id: &str,
    ) -> Result<Option<String>, StorageError>;

    /// 设置当前有效 refresh token 的 jti（设置为 None 表示撤销所有 refresh token）。
    async fn set_refresh_jti(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        refresh_jti: Option<&str>,
    ) -> Result<bool, StorageError>;
}

/// RBAC 管理接口（tenant 级）
#[async_trait]
pub trait RbacStore: Send + Sync {
    async fn list_users(&self, ctx: &TenantContext) -> Result<Vec<RbacUserRecord>, StorageError>;

    async fn create_user(
        &self,
        ctx: &TenantContext,
        record: RbacUserCreate,
    ) -> Result<RbacUserRecord, StorageError>;

    async fn update_user(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        update: RbacUserUpdate,
    ) -> Result<Option<RbacUserRecord>, StorageError>;

    async fn set_user_roles(
        &self,
        ctx: &TenantContext,
        user_id: &str,
        roles: Vec<String>,
    ) -> Result<Option<RbacUserRecord>, StorageError>;

    async fn list_roles(&self, ctx: &TenantContext) -> Result<Vec<RbacRoleRecord>, StorageError>;

    async fn create_role(
        &self,
        ctx: &TenantContext,
        record: RbacRoleCreate,
    ) -> Result<RbacRoleRecord, StorageError>;

    async fn delete_role(&self, ctx: &TenantContext, role_code: &str)
    -> Result<bool, StorageError>;

    async fn set_role_permissions(
        &self,
        ctx: &TenantContext,
        role_code: &str,
        permissions: Vec<String>,
    ) -> Result<Option<RbacRoleRecord>, StorageError>;

    async fn list_permissions(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<PermissionRecord>, StorageError>;
}

/// 项目存储接口
///
/// 提供项目 CRUD 操作和租户归属校验。
#[async_trait]
pub trait ProjectStore: Send + Sync {
    /// 列出当前租户的所有项目
    async fn list_projects(&self, ctx: &TenantContext) -> Result<Vec<ProjectRecord>, StorageError>;

    /// 查找指定项目
    async fn find_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Option<ProjectRecord>, StorageError>;

    /// 创建新项目
    async fn create_project(
        &self,
        ctx: &TenantContext,
        record: ProjectRecord,
    ) -> Result<ProjectRecord, StorageError>;

    /// 更新项目
    async fn update_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        update: ProjectUpdate,
    ) -> Result<Option<ProjectRecord>, StorageError>;

    /// 删除项目
    async fn delete_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<bool, StorageError>;

    /// 验证项目归属当前租户
    async fn project_belongs_to_tenant(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<bool, StorageError>;
}

// ============================================================================
// 楼宇层级存储接口（区域 → 楼宇 → 楼层 → 房间）
// ============================================================================

/// 区域存储接口
#[async_trait]
pub trait AreaStore: Send + Sync {
    async fn list_areas(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<AreaRecord>, StorageError>;

    async fn find_area(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        area_id: &str,
    ) -> Result<Option<AreaRecord>, StorageError>;

    async fn create_area(
        &self,
        ctx: &TenantContext,
        record: AreaRecord,
    ) -> Result<AreaRecord, StorageError>;

    async fn update_area(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        area_id: &str,
        update: AreaUpdate,
    ) -> Result<Option<AreaRecord>, StorageError>;

    async fn delete_area(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        area_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 楼宇存储接口
#[async_trait]
pub trait BuildingStore: Send + Sync {
    async fn list_buildings(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        area_id: Option<&str>,
    ) -> Result<Vec<BuildingRecord>, StorageError>;

    async fn find_building(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        building_id: &str,
    ) -> Result<Option<BuildingRecord>, StorageError>;

    async fn create_building(
        &self,
        ctx: &TenantContext,
        record: BuildingRecord,
    ) -> Result<BuildingRecord, StorageError>;

    async fn update_building(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        building_id: &str,
        update: BuildingUpdate,
    ) -> Result<Option<BuildingRecord>, StorageError>;

    async fn delete_building(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        building_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 楼层存储接口
#[async_trait]
pub trait FloorStore: Send + Sync {
    async fn list_floors(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        building_id: Option<&str>,
    ) -> Result<Vec<FloorRecord>, StorageError>;

    async fn find_floor(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        floor_id: &str,
    ) -> Result<Option<FloorRecord>, StorageError>;

    async fn create_floor(
        &self,
        ctx: &TenantContext,
        record: FloorRecord,
    ) -> Result<FloorRecord, StorageError>;

    async fn update_floor(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        floor_id: &str,
        update: FloorUpdate,
    ) -> Result<Option<FloorRecord>, StorageError>;

    async fn delete_floor(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        floor_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 房间存储接口
#[async_trait]
pub trait RoomStore: Send + Sync {
    async fn list_rooms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        floor_id: Option<&str>,
    ) -> Result<Vec<RoomRecord>, StorageError>;

    async fn find_room(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        room_id: &str,
    ) -> Result<Option<RoomRecord>, StorageError>;

    async fn create_room(
        &self,
        ctx: &TenantContext,
        record: RoomRecord,
    ) -> Result<RoomRecord, StorageError>;

    async fn update_room(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        room_id: &str,
        update: RoomUpdate,
    ) -> Result<Option<RoomRecord>, StorageError>;

    async fn delete_room(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        room_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 网关存储接口
///
/// 提供网关 CRUD 操作。
#[async_trait]
pub trait GatewayStore: Send + Sync {
    /// 列出指定项目的所有网关
    async fn list_gateways(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<GatewayRecord>, StorageError>;

    /// 查找指定网关
    async fn find_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<Option<GatewayRecord>, StorageError>;

    /// 创建新网关
    async fn create_gateway(
        &self,
        ctx: &TenantContext,
        record: GatewayRecord,
    ) -> Result<GatewayRecord, StorageError>;

    /// 更新网关
    async fn update_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
        update: GatewayUpdate,
    ) -> Result<Option<GatewayRecord>, StorageError>;

    /// 删除网关
    async fn delete_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 设备存储接口
///
/// 提供设备 CRUD 操作。
#[async_trait]
pub trait DeviceStore: Send + Sync {
    /// 列出指定项目的所有设备
    async fn list_devices(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<DeviceRecord>, StorageError>;

    /// 查找指定设备
    async fn find_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<DeviceRecord>, StorageError>;

    /// 创建新设备
    async fn create_device(
        &self,
        ctx: &TenantContext,
        record: DeviceRecord,
    ) -> Result<DeviceRecord, StorageError>;

    /// 更新设备
    async fn update_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        update: DeviceUpdate,
    ) -> Result<Option<DeviceRecord>, StorageError>;

    /// 删除设备
    async fn delete_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 点位存储接口
///
/// 提供点位 CRUD 操作。
#[async_trait]
pub trait PointStore: Send + Sync {
    /// 列出指定项目的所有点
    async fn list_points(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<PointRecord>, StorageError>;

    /// 查找指定点
    async fn find_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<PointRecord>, StorageError>;

    /// 创建新点
    async fn create_point(
        &self,
        ctx: &TenantContext,
        record: PointRecord,
    ) -> Result<PointRecord, StorageError>;

    /// 更新点
    async fn update_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        update: PointUpdate,
    ) -> Result<Option<PointRecord>, StorageError>;

    /// 删除点
    async fn delete_point(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 点映射存储接口
///
/// 提供点映射 CRUD 操作。
#[async_trait]
pub trait PointMappingStore: Send + Sync {
    /// 列出指定项目的所有点映射
    async fn list_point_mappings(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<PointMappingRecord>, StorageError>;

    /// 查找指定点映射
    async fn find_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
    ) -> Result<Option<PointMappingRecord>, StorageError>;

    /// 创建新点映射
    async fn create_point_mapping(
        &self,
        ctx: &TenantContext,
        record: PointMappingRecord,
    ) -> Result<PointMappingRecord, StorageError>;

    /// 更新点映射
    async fn update_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
        update: PointMappingUpdate,
    ) -> Result<Option<PointMappingRecord>, StorageError>;

    /// 删除点映射
    async fn delete_point_mapping(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        source_id: &str,
    ) -> Result<bool, StorageError>;
}

/// 时序写入接口
///
/// 用于写入 Timescale measurement 数据。
#[async_trait]
pub trait MeasurementStore: Send + Sync {
    /// 写入单条测点值
    async fn write_measurement(
        &self,
        ctx: &TenantContext,
        value: &PointValue,
    ) -> Result<(), StorageError>;

    /// 批量写入测点值
    async fn write_measurements(
        &self,
        ctx: &TenantContext,
        values: &[PointValue],
    ) -> Result<usize, StorageError>;

    /// 查询参数（支持 keyset 分页与聚合）。
    async fn query_measurements(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        options: MeasurementsQueryOptions,
    ) -> Result<Vec<MeasurementRecord>, StorageError>;

    /// 查询历史测点值
    async fn list_measurements(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        from_ms: Option<i64>,
        to_ms: Option<i64>,
        limit: i64,
    ) -> Result<Vec<MeasurementRecord>, StorageError> {
        self.query_measurements(
            ctx,
            project_id,
            point_id,
            MeasurementsQueryOptions::simple(from_ms, to_ms, limit),
        )
        .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeOrder {
    Asc,
    Desc,
}

impl Default for TimeOrder {
    fn default() -> Self {
        Self::Asc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementAggFn {
    Avg,
    Min,
    Max,
    Sum,
    Count,
}

#[derive(Debug, Clone, Copy)]
pub struct MeasurementAggregation {
    pub bucket_ms: i64,
    pub func: MeasurementAggFn,
}

#[derive(Debug, Clone, Copy)]
pub struct MeasurementsQueryOptions {
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub cursor_ts_ms: Option<i64>,
    pub order: TimeOrder,
    pub limit: i64,
    pub aggregation: Option<MeasurementAggregation>,
}

impl MeasurementsQueryOptions {
    pub fn simple(from_ms: Option<i64>, to_ms: Option<i64>, limit: i64) -> Self {
        Self {
            from_ms,
            to_ms,
            cursor_ts_ms: None,
            order: TimeOrder::Asc,
            limit,
            aggregation: None,
        }
    }
}

/// 实时数据接口
///
/// 用于维护 Redis last_value。
#[async_trait]
pub trait RealtimeStore: Send + Sync {
    /// 写入或更新点位 last_value
    async fn upsert_last_value(
        &self,
        ctx: &TenantContext,
        value: &PointValue,
    ) -> Result<(), StorageError>;

    /// 查询单个点位 last_value
    async fn get_last_value(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<RealtimeRecord>, StorageError>;

    /// 查询项目内全部 last_value
    async fn list_last_values(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<RealtimeRecord>, StorageError>;
}

/// 控制命令存储接口
#[async_trait]
pub trait CommandStore: Send + Sync {
    /// 创建命令
    async fn create_command(
        &self,
        ctx: &TenantContext,
        record: CommandRecord,
    ) -> Result<CommandRecord, StorageError>;

    /// 更新命令状态
    async fn update_command_status(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
        status: &str,
    ) -> Result<Option<CommandRecord>, StorageError>;

    /// 条件更新命令状态（用于超时等幂等状态流转）。
    ///
    /// 仅当当前状态等于 `from_status` 时，才更新为 `to_status`。
    /// 返回 true 表示更新成功（状态已流转）。
    async fn transition_command_status(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
        from_status: &str,
        to_status: &str,
    ) -> Result<bool, StorageError>;

    /// 查询命令列表
    async fn list_commands(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<CommandRecord>, StorageError>;
}

/// 命令回执存储接口
#[async_trait]
pub trait CommandReceiptStore: Send + Sync {
    /// 写入命令回执
    async fn create_receipt(
        &self,
        ctx: &TenantContext,
        record: CommandReceiptRecord,
    ) -> Result<CommandReceiptWriteResult, StorageError>;

    /// 查询命令回执
    async fn list_receipts(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        command_id: &str,
    ) -> Result<Vec<CommandReceiptRecord>, StorageError>;
}

/// 命令回执写入结果（用于幂等处理）。
#[derive(Debug, Clone)]
pub struct CommandReceiptWriteResult {
    pub record: CommandReceiptRecord,
    pub inserted: bool,
}

/// 审计日志存储接口
#[async_trait]
pub trait AuditLogStore: Send + Sync {
    /// 写入审计日志
    async fn create_audit_log(
        &self,
        ctx: &TenantContext,
        record: AuditLogRecord,
    ) -> Result<AuditLogRecord, StorageError>;

    /// 查询审计日志
    async fn list_audit_logs(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        from_ms: Option<i64>,
        to_ms: Option<i64>,
        limit: i64,
    ) -> Result<Vec<AuditLogRecord>, StorageError>;
}
