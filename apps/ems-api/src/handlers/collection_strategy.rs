//! 采集策略 handlers
//!
//! 提供采集策略的管理接口：
//! - GET /projects/{id}/collection-strategies - 列出项目的采集策略
//! - POST /projects/{id}/collection-strategies - 批量创建/更新采集策略
//! - DELETE /projects/{id}/collection-strategies/{sid} - 删除采集策略
//! - POST /projects/{id}/points/{pid}/test - 测试点位连接
//! - GET /points/batch - 批量获取多项目点位
//! - POST /projects/{id}/collection-strategies/batch-enabled - 批量更新启用状态

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope, require_tenant_context};
use crate::utils::response::{bad_request_error, not_found_error, storage_error};
use api_contract::{
    ApiResponse, BatchPointsQuery, BatchUpdateEnabledRequest, BatchUpsertStrategiesRequest,
    CollectionStrategyDto, PointTestResultDto,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use ems_storage::CollectionStrategyCreate;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

#[derive(serde::Deserialize)]
pub struct StrategyPath {
    project_id: String,
    strategy_id: String,
}

#[derive(serde::Deserialize)]
pub struct PointPath {
    project_id: String,
    point_id: String,
}

/// 策略记录转 DTO
fn strategy_to_dto(record: ems_storage::CollectionStrategyRecord) -> CollectionStrategyDto {
    CollectionStrategyDto {
        strategy_id: record.strategy_id,
        project_id: record.project_id,
        point_id: record.point_id,
        enabled: record.enabled,
        interval_value: record.interval_value,
        interval_unit: record.interval_unit,
        write_to_db: record.write_to_db,
        last_collected_at_ms: record.last_collected_at,
        last_value: record.last_value,
        last_error: record.last_error,
    }
}

/// 列出项目的采集策略
pub async fn list_strategies(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::DATA_REALTIME_READ) {
        return response;
    }

    match state
        .collection_strategy_store
        .list_strategies(&ctx, &path.project_id)
        .await
    {
        Ok(items) => {
            let data: Vec<CollectionStrategyDto> = items.into_iter().map(strategy_to_dto).collect();
            (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
        }
        Err(err) => storage_error(err),
    }
}

/// 批量获取多项目点位（用于采集策略页面）
pub async fn list_points_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<BatchPointsQuery>,
) -> Response {
    let ctx = match require_tenant_context(&state, &headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    // 解析项目 ID 列表（支持逗号分隔）
    let project_ids: Vec<String> = query
        .project_ids
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    if project_ids.is_empty() {
        return bad_request_error("projectIds is required");
    }

    // 批量获取点位
    let mut all_points = Vec::new();
    for project_id in &project_ids {
        match state.point_store.list_points(&ctx, project_id).await {
            Ok(points) => {
                for p in points {
                    all_points.push(api_contract::PointDto {
                        point_id: p.point_id,
                        project_id: p.project_id,
                        device_id: p.device_id,
                        key: p.key,
                        data_type: p.data_type,
                        unit: p.unit,
                        protocol_detail: p.protocol_detail,
                    });
                }
            }
            Err(err) => return storage_error(err),
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(all_points))).into_response()
}

/// 批量创建/更新采集策略
pub async fn upsert_strategies(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<BatchUpsertStrategiesRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }

    let mut results = Vec::new();
    for strategy in req.strategies {
        let record = CollectionStrategyCreate {
            strategy_id: Uuid::new_v4().to_string(),
            tenant_id: ctx.tenant_id.clone(),
            project_id: path.project_id.clone(),
            point_id: strategy.point_id,
            enabled: strategy.enabled,
            interval_value: strategy.interval_value,
            interval_unit: strategy.interval_unit,
            write_to_db: strategy.write_to_db,
        };

        match state
            .collection_strategy_store
            .upsert_strategy(&ctx, record)
            .await
        {
            Ok(item) => results.push(strategy_to_dto(item)),
            Err(err) => return storage_error(err),
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(results))).into_response()
}

/// 删除采集策略
pub async fn delete_strategy(
    State(state): State<AppState>,
    Path(path): Path<StrategyPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }

    match state
        .collection_strategy_store
        .delete_strategy(&ctx, &path.project_id, &path.strategy_id)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(false) => not_found_error(),
        Err(err) => storage_error(err),
    }
}

/// 批量更新启用状态
pub async fn batch_update_enabled(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<BatchUpdateEnabledRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::ASSET_POINT_WRITE) {
        return response;
    }

    match state
        .collection_strategy_store
        .batch_update_enabled(&ctx, &req.strategy_ids, req.enabled)
        .await
    {
        Ok(count) => (
            StatusCode::OK,
            Json(ApiResponse::success(
                serde_json::json!({ "updated": count }),
            )),
        )
            .into_response(),
        Err(err) => storage_error(err),
    }
}

/// 测试点位连接
///
/// 通过协议层直接连接设备读取数据，用于验证配置正确性。
pub async fn test_point(
    State(state): State<AppState>,
    Path(path): Path<PointPath>,
    headers: HeaderMap,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    if let Err(response) = require_permission(&ctx, permissions::DATA_REALTIME_READ) {
        return response;
    }

    // 获取点位信息
    let point = match state
        .point_store
        .find_point(&ctx, &path.project_id, &path.point_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => return not_found_error(),
        Err(err) => return storage_error(err),
    };

    // 获取设备信息
    let device = match state
        .device_store
        .find_device(&ctx, &path.project_id, &point.device_id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return bad_request_error("device not found"),
        Err(err) => return storage_error(err),
    };

    // 获取网关信息
    let gateway = match state
        .gateway_store
        .find_gateway(&ctx, &path.project_id, &device.gateway_id)
        .await
    {
        Ok(Some(g)) => g,
        Ok(None) => return bad_request_error("gateway not found"),
        Err(err) => return storage_error(err),
    };

    let start = std::time::Instant::now();
    let mut result = PointTestResultDto {
        success: false,
        point_id: point.point_id.clone(),
        value: None,
        error: None,
        latency_ms: 0,
    };

    if gateway.protocol_type == "modbus_tcp" {
        use ems_protocol::ModbusTcpSource;

        let protocol_config = gateway.protocol_config.as_deref().unwrap_or("{}");
        let config_res = ModbusTcpSource::from_json(protocol_config);
        match config_res {
            Ok(mut source) => {
                let address_config = device.address_config.as_deref().unwrap_or("{}");
                let protocol_detail = point.protocol_detail.as_deref().unwrap_or("{}");

                let task_res = source.add_task_from_config(
                    &ctx.tenant_id,
                    &point.project_id,
                    &gateway.gateway_id,
                    &device.device_id,
                    &point.point_id,
                    address_config,
                    protocol_detail,
                    None,
                    None,
                );

                match task_res {
                    Ok(_) => match source.test_once().await {
                        Ok(events) => {
                            if let Some(event) = events.first() {
                                result.success = true;
                                result.value = Some(event.value.to_string());
                            } else {
                                result.error = Some("No data received".to_string());
                            }
                        }
                        Err(e) => {
                            result.error = Some(format!("Connection Error: {}", e));
                        }
                    },
                    Err(e) => {
                        result.error = Some(format!("Invalid configuration: {}", e));
                    }
                }
            }
            Err(e) => {
                result.error = Some(format!("Invalid gateway config: {}", e));
            }
        }
    } else {
        result.error = Some(format!("Unsupported protocol: {}", gateway.protocol_type));
    }

    result.latency_ms = start.elapsed().as_millis() as i64;
    (StatusCode::OK, Json(ApiResponse::success(result))).into_response()
}
