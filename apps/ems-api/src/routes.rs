//! 路由定义
//!
//! 集中管理所有 API 路由，将路径映射到对应的 handlers。
//! 路由包括：
//! - 健康检查：/health
//! - 认证接口：/login, /refresh-token, /get-async-routes
//! - 项目管理：/projects/*
//! - 网关管理：/projects/{id}/gateways/*
//! - 设备管理：/projects/{id}/devices/*
//! - 点管理：/projects/{id}/points/*
//! - 点映射管理：/projects/{id}/point-mappings/*

use super::AppState;
use super::handlers::*;
use axum::{
    Router,
    routing::{get, post},
};

/// 创建 API 路由
///
/// 返回包含所有 API 端点的 Router，支持 / 和 /api/ 两种前缀
pub fn create_api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/refresh-token", post(refresh_token))
        .route("/get-async-routes", get(get_async_routes))
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/:project_id",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route(
            "/projects/:project_id/gateways",
            get(list_gateways).post(create_gateway),
        )
        .route(
            "/projects/:project_id/gateways/:gateway_id",
            get(get_gateway).put(update_gateway).delete(delete_gateway),
        )
        .route(
            "/projects/:project_id/devices",
            get(list_devices).post(create_device),
        )
        .route(
            "/projects/:project_id/devices/:device_id",
            get(get_device).put(update_device).delete(delete_device),
        )
        .route(
            "/projects/:project_id/points",
            get(list_points).post(create_point),
        )
        .route(
            "/projects/:project_id/points/:point_id",
            get(get_point).put(update_point).delete(delete_point),
        )
        .route(
            "/projects/:project_id/point-mappings",
            get(list_point_mappings).post(create_point_mapping),
        )
        .route(
            "/projects/:project_id/point-mappings/:source_id",
            get(get_point_mapping)
                .put(update_point_mapping)
                .delete(delete_point_mapping),
        )
}
