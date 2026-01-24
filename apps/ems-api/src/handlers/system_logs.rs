//! 系统日志 Handlers
//!
//! - GET /projects/{id}/system-logs
//! - GET /projects/{id}/system-logs/unread
//! - POST /projects/{id}/system-logs/read

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::{bad_request_error, storage_error};
use api_contract::{ApiResponse, MarkReadRequest, SystemLogDto, SystemLogQuery, UnreadStatsDto};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_storage::models::LogCategory;

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

/// 查询系统日志
pub async fn list_system_logs(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    Query(query): Query<SystemLogQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    // 权限检查：视具体需求，这里暂定需要 READ 权限，或者复用 project.read
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_READ) {
        return response;
    }

    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let storage_query = ems_storage::models::SystemLogQuery {
        category: query.category.as_deref().and_then(LogCategory::parse),
        level: query
            .level
            .as_deref()
            .and_then(ems_storage::models::LogLevel::parse),
        unread_only: query.unread_only.unwrap_or(false),
        from_ms: query.from,
        to_ms: query.to,
        limit,
    };

    match state
        .system_log_store
        .list_system_logs(&ctx, storage_query)
        .await
    {
        Ok(logs) => {
            let dtos: Vec<SystemLogDto> = logs
                .into_iter()
                .map(|log| SystemLogDto {
                    log_id: log.log_id,
                    tenant_id: log.tenant_id,
                    project_id: log.project_id,
                    category: log.category.as_str().to_string(),
                    level: log.level.as_str().to_string(),
                    title: log.title,
                    message: log.message,
                    source: log.source,
                    resource: log.resource,
                    actor: log.actor,
                    metadata: log.metadata.and_then(|s| serde_json::from_str(&s).ok()),
                    is_read: log.is_read,
                    created_at_ms: log.created_at_ms,
                })
                .collect();
            (StatusCode::OK, Json(ApiResponse::success(dtos))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 获取未读数量
pub async fn get_unread_count(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    Query(query): Query<SystemLogQuery>, // Reusing Query struct for category
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    if let Err(response) = require_permission(&ctx, permissions::PROJECT_READ) {
        return response;
    }

    let category = query.category.as_deref().and_then(LogCategory::parse);

    match state
        .system_log_store
        .get_unread_count(&ctx, category)
        .await
    {
        Ok(stats) => {
            let dto = UnreadStatsDto {
                operation: stats.operation,
                error: stats.error,
                warning: stats.warning,
                total: stats.total,
            };
            (StatusCode::OK, Json(ApiResponse::success(dto))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 标记已读
pub async fn mark_as_read(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(body): Json<MarkReadRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    // 权限检查：可能需要 WRITE 权限
    if let Err(response) = require_permission(&ctx, permissions::PROJECT_READ) {
        return response;
    }

    if body.log_ids.is_empty() {
        return bad_request_error("log_ids cannot be empty");
    }

    match state
        .system_log_store
        .mark_as_read(&ctx, body.log_ids)
        .await
    {
        Ok(count) => (StatusCode::OK, Json(ApiResponse::success(count))).into_response(),
        Err(err) => storage_error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
    use domain::TenantContext;
    use ems_auth::{AuthService, JwtManager};
    use ems_storage::{
        InMemoryProjectStore, InMemoryUserStore, ProjectStore,
        models::{LogCategory, LogLevel, SystemLogRecord},
    };
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_state() -> AppState {
        let user_store = Arc::new(InMemoryUserStore::with_default_admin());
        let jwt = JwtManager::new("secret".to_string(), 3600, 3600);
        let auth = Arc::new(AuthService::new(user_store.clone(), jwt));
        let project_store: Arc<dyn ProjectStore> =
            Arc::new(InMemoryProjectStore::with_default_project()); // Contains "project-1"

        // Mock other stores
        let command_store = Arc::new(ems_storage::InMemoryCommandStore::new());
        let command_receipt_store = Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
        let audit_log_store = Arc::new(ems_storage::InMemoryAuditLogStore::new());
        let dispatcher = Arc::new(ems_control::NoopDispatcher);
        let command_service = Arc::new(ems_control::CommandService::new(
            command_store.clone(),
            audit_log_store.clone(),
            dispatcher,
        ));

        AppState {
            auth,
            db_pool: None,
            rbac_store: user_store,
            project_store,
            gateway_store: Arc::new(ems_storage::InMemoryGatewayStore::new()),
            device_store: Arc::new(ems_storage::InMemoryDeviceStore::new()),
            point_store: Arc::new(ems_storage::InMemoryPointStore::new()),
            point_mapping_store: Arc::new(ems_storage::InMemoryPointMappingStore::new()),
            measurement_store: Arc::new(ems_storage::InMemoryMeasurementStore::new()),
            realtime_store: Arc::new(ems_storage::InMemoryRealtimeStore::new()),
            online_store: Arc::new(ems_storage::InMemoryOnlineStore::new()),
            command_store,
            command_receipt_store,
            audit_log_store,
            collection_strategy_store: Arc::new(ems_storage::InMemoryCollectionStrategyStore::new()),
            system_log_store: Arc::new(ems_storage::InMemorySystemLogStore::new()),
            command_service,
        }
    }

    async fn get_auth_headers(state: &AppState) -> HeaderMap {
        let (_, tokens) = state.auth.login("admin", "admin123").await.expect("login");
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", tokens.access_token)).expect("header"),
        );
        headers
    }

    fn create_mock_ctx() -> TenantContext {
        TenantContext::new(
            "tenant-1".to_string(),
            "user-1".to_string(),
            vec![],
            vec![],
            None,
        )
    }

    #[tokio::test]
    async fn test_list_system_logs_empty() {
        let state = create_test_state();
        let headers = get_auth_headers(&state).await;

        let path = ProjectPath {
            project_id: "project-1".to_string(),
        };
        let query = SystemLogQuery {
            from: None,
            to: None,
            category: None,
            level: None,
            unread_only: None,
            limit: None,
        };

        let response = list_system_logs(State(state), Path(path), Query(query), headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_and_list_system_logs() {
        let state = create_test_state();
        let headers = get_auth_headers(&state).await;
        let ctx = create_mock_ctx();

        let record = SystemLogRecord {
            log_id: "log-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            project_id: Some("project-1".to_string()),
            category: LogCategory::Error,
            level: LogLevel::Error,
            title: "Test Error".to_string(),
            message: "Something went wrong".to_string(),
            source: None,
            resource: None,
            actor: None,
            metadata: None,
            is_read: false,
            created_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            read_at_ms: None,
        };
        // FIXED: passed record by value
        state
            .system_log_store
            .create_system_log(&ctx, record)
            .await
            .expect("create log");

        let path = ProjectPath {
            project_id: "project-1".to_string(),
        };
        let query = SystemLogQuery {
            from: None,
            to: None,
            category: Some("error".to_string()),
            level: None,
            unread_only: None,
            limit: None,
        };

        let response = list_system_logs(State(state), Path(path), Query(query), headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_unread_count() {
        let state = create_test_state();
        let headers = get_auth_headers(&state).await;
        let ctx = create_mock_ctx();

        let record = SystemLogRecord {
            log_id: "log-unread".to_string(),
            tenant_id: "tenant-1".to_string(),
            project_id: Some("project-1".to_string()),
            category: LogCategory::Warning,
            level: LogLevel::Warn,
            title: "Test Warning".to_string(),
            message: "Warning msg".to_string(),
            source: None,
            resource: None,
            actor: None,
            metadata: None,
            is_read: false,
            created_at_ms: 1000,
            read_at_ms: None,
        };
        // FIXED: passed record by value
        state
            .system_log_store
            .create_system_log(&ctx, record)
            .await
            .expect("create log");

        let path = ProjectPath {
            project_id: "project-1".to_string(),
        };
        let query = SystemLogQuery {
            from: None,
            to: None,
            category: None,
            level: None,
            unread_only: None,
            limit: None,
        };

        let response = get_unread_count(State(state), Path(path), Query(query), headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
