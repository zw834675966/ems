//! 存储接口 Trait 定义
//!
//! 定义所有资源存储的异步接口：
//! - UserStore：用户存储
//! - ProjectStore：项目存储
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
    DeviceRecord, DeviceUpdate, GatewayRecord, GatewayUpdate, PointMappingRecord,
    PointMappingUpdate, PointRecord, PointUpdate, ProjectRecord, ProjectUpdate, UserRecord,
};
use async_trait::async_trait;
use domain::TenantContext;

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
