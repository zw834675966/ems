use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use ems_api::{AppState, create_api_router};
use ems_auth::{AuthService, JwtManager};
use ems_control::{CommandService, NoopDispatcher};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

fn build_state() -> AppState {
    let user_store: Arc<ems_storage::InMemoryUserStore> =
        Arc::new(ems_storage::InMemoryUserStore::with_default_admin());
    let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
    let auth: Arc<AuthService> = Arc::new(AuthService::new(user_store.clone(), jwt));
    let rbac_store: Arc<dyn ems_storage::RbacStore> = user_store.clone();

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
    let measurement_store: Arc<dyn ems_storage::MeasurementStore> =
        Arc::new(ems_storage::InMemoryMeasurementStore::new());
    let realtime_store: Arc<dyn ems_storage::RealtimeStore> =
        Arc::new(ems_storage::InMemoryRealtimeStore::new());
    let online_store: Arc<dyn ems_storage::OnlineStore> =
        Arc::new(ems_storage::InMemoryOnlineStore::new());
    let command_store: Arc<dyn ems_storage::CommandStore> =
        Arc::new(ems_storage::InMemoryCommandStore::new());
    let command_receipt_store: Arc<dyn ems_storage::CommandReceiptStore> =
        Arc::new(ems_storage::InMemoryCommandReceiptStore::new());
    let audit_log_store: Arc<dyn ems_storage::AuditLogStore> =
        Arc::new(ems_storage::InMemoryAuditLogStore::new());
    let command_service = Arc::new(CommandService::new(
        command_store.clone(),
        audit_log_store.clone(),
        Arc::new(NoopDispatcher),
    ));
    let collection_strategy_store: Arc<dyn ems_storage::CollectionStrategyStore> =
        Arc::new(ems_storage::InMemoryCollectionStrategyStore::new());
    let system_log_store: Arc<dyn ems_storage::SystemLogStore> =
        Arc::new(ems_storage::InMemorySystemLogStore::default());

    AppState::new(
        auth,
        None,
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
    )
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json")
}

#[tokio::test]
async fn login_create_project_and_enforce_tenant_scope() {
    unsafe { std::env::set_var("EMS_JWT_SECRET", "test-secret") };
    let state = build_state();
    let app = Router::new()
        .merge(create_api_router())
        .with_state(state.clone());

    let login_req = Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"admin","password":"admin123"}"#))
        .expect("login request");
    let login_res = app.clone().oneshot(login_req).await.expect("login");
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_json = json_body(login_res).await;
    let access_token = login_json["data"]["accessToken"]
        .as_str()
        .expect("access token");

    let create_req = Request::builder()
        .method("POST")
        .uri("/projects")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", access_token))
        .body(Body::from(r#"{"name":"Project A","timezone":"UTC"}"#))
        .expect("create request");
    let create_res = app.clone().oneshot(create_req).await.expect("create");
    assert_eq!(create_res.status(), StatusCode::OK);
    let create_json = json_body(create_res).await;
    let project_id = create_json["data"]["projectId"]
        .as_str()
        .expect("project id")
        .to_string();

    let jwt = JwtManager::new("test-secret".to_string(), 3600, 7200);
    let other_ctx = domain::TenantContext::new(
        "tenant-2",
        "user-2",
        Vec::new(),
        vec![domain::permissions::PROJECT_READ.to_string()],
        None,
    );
    let other_token = jwt.issue_tokens(&other_ctx).expect("tokens").access_token;

    let forbidden_req = Request::builder()
        .method("GET")
        .uri(format!("/projects/{}", project_id))
        .header("authorization", format!("Bearer {}", other_token))
        .body(Body::empty())
        .expect("tenant mismatch request");
    let forbidden_res = app.clone().oneshot(forbidden_req).await.expect("forbidden");
    assert_eq!(forbidden_res.status(), StatusCode::NOT_FOUND);

    let ok_req = Request::builder()
        .method("GET")
        .uri(format!("/projects/{}", project_id))
        .header("authorization", format!("Bearer {}", access_token))
        .body(Body::empty())
        .expect("get project request");
    let ok_res = app.clone().oneshot(ok_req).await.expect("get project");
    assert_eq!(ok_res.status(), StatusCode::OK);
}
