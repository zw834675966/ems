use std::sync::Arc;

use ems_auth::AuthService;
use ems_control::CommandService;

/// 应用状态（AppState）
///
/// 这是整个 API 服务的核心状态容器，包含所有业务模块所需的服务和存储层实例。
/// 该结构体会被注入到每个 HTTP 请求处理器中，通过 Axum 的 `State` 提取器访问。
#[derive(Clone)]
pub struct AppState {
    pub(crate) auth: Arc<AuthService>,
    pub(crate) db_pool: Option<sqlx::PgPool>,
    pub(crate) rbac_store: Arc<dyn ems_storage::RbacStore>,
    pub(crate) project_store: Arc<dyn ems_storage::ProjectStore>,
    pub(crate) gateway_store: Arc<dyn ems_storage::GatewayStore>,
    pub(crate) device_store: Arc<dyn ems_storage::DeviceStore>,
    pub(crate) point_store: Arc<dyn ems_storage::PointStore>,
    pub(crate) point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
    pub(crate) measurement_store: Arc<dyn ems_storage::MeasurementStore>,
    pub(crate) realtime_store: Arc<dyn ems_storage::RealtimeStore>,
    pub(crate) online_store: Arc<dyn ems_storage::OnlineStore>,
    pub(crate) command_store: Arc<dyn ems_storage::CommandStore>,
    pub(crate) command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore>,
    pub(crate) audit_log_store: Arc<dyn ems_storage::AuditLogStore>,
    pub(crate) command_service: Arc<CommandService>,
    pub(crate) collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore>,
    pub(crate) system_log_store: Arc<dyn ems_storage::SystemLogStore>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        auth: Arc<AuthService>,
        db_pool: Option<sqlx::PgPool>,
        rbac_store: Arc<dyn ems_storage::RbacStore>,
        project_store: Arc<dyn ems_storage::ProjectStore>,
        gateway_store: Arc<dyn ems_storage::GatewayStore>,
        device_store: Arc<dyn ems_storage::DeviceStore>,
        point_store: Arc<dyn ems_storage::PointStore>,
        point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
        measurement_store: Arc<dyn ems_storage::MeasurementStore>,
        realtime_store: Arc<dyn ems_storage::RealtimeStore>,
        online_store: Arc<dyn ems_storage::OnlineStore>,
        command_store: Arc<dyn ems_storage::CommandStore>,
        command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore>,
        audit_log_store: Arc<dyn ems_storage::AuditLogStore>,
        command_service: Arc<CommandService>,
        collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore>,
        system_log_store: Arc<dyn ems_storage::SystemLogStore>,
    ) -> Self {
        Self {
            auth,
            db_pool,
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
            collection_strategy_store,
            system_log_store,
        }
    }
}
