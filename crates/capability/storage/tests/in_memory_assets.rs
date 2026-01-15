use domain::TenantContext;
use ems_storage::{
    DeviceRecord, DeviceStore, GatewayRecord, GatewayStore, InMemoryDeviceStore,
    InMemoryGatewayStore, InMemoryPointMappingStore, InMemoryPointStore, PointMappingRecord,
    PointMappingStore, PointRecord, PointStore,
};

fn tenant_ctx(project_id: &str) -> TenantContext {
    TenantContext::new(
        "tenant-1",
        "user-1",
        vec![],
        vec![],
        Some(project_id.to_string()),
    )
}

#[tokio::test]
async fn gateway_in_memory_crud() {
    let store = InMemoryGatewayStore::new();
    let ctx = tenant_ctx("project-1");
    let record = GatewayRecord {
        gateway_id: "gw-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        project_id: "project-1".to_string(),
        name: "Gateway 1".to_string(),
        status: "offline".to_string(),
    };
    let created = store.create_gateway(&ctx, record).await.expect("create");
    assert_eq!(created.gateway_id, "gw-1");

    let list = store.list_gateways(&ctx, "project-1").await.expect("list");
    assert_eq!(list.len(), 1);

    let got = store
        .find_gateway(&ctx, "project-1", "gw-1")
        .await
        .expect("find");
    assert!(got.is_some());
}

#[tokio::test]
async fn device_in_memory_crud() {
    let store = InMemoryDeviceStore::new();
    let ctx = tenant_ctx("project-1");
    let record = DeviceRecord {
        device_id: "dev-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        project_id: "project-1".to_string(),
        gateway_id: "gw-1".to_string(),
        name: "Device 1".to_string(),
        model: Some("m1".to_string()),
    };
    let created = store.create_device(&ctx, record).await.expect("create");
    assert_eq!(created.device_id, "dev-1");

    let list = store.list_devices(&ctx, "project-1").await.expect("list");
    assert_eq!(list.len(), 1);

    let got = store
        .find_device(&ctx, "project-1", "dev-1")
        .await
        .expect("find");
    assert!(got.is_some());
}

#[tokio::test]
async fn point_in_memory_crud() {
    let store = InMemoryPointStore::new();
    let ctx = tenant_ctx("project-1");
    let record = PointRecord {
        point_id: "pt-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        project_id: "project-1".to_string(),
        device_id: "dev-1".to_string(),
        key: "temp".to_string(),
        data_type: "float".to_string(),
        unit: Some("C".to_string()),
    };
    let created = store.create_point(&ctx, record).await.expect("create");
    assert_eq!(created.point_id, "pt-1");

    let list = store.list_points(&ctx, "project-1").await.expect("list");
    assert_eq!(list.len(), 1);

    let got = store
        .find_point(&ctx, "project-1", "pt-1")
        .await
        .expect("find");
    assert!(got.is_some());
}

#[tokio::test]
async fn point_mapping_in_memory_crud() {
    let store = InMemoryPointMappingStore::new();
    let ctx = tenant_ctx("project-1");
    let record = PointMappingRecord {
        source_id: "src-1".to_string(),
        tenant_id: "tenant-1".to_string(),
        project_id: "project-1".to_string(),
        point_id: "pt-1".to_string(),
        source_type: "mqtt".to_string(),
        address: "topic/1".to_string(),
        scale: Some(1.0),
        offset: Some(0.0),
    };
    let created = store
        .create_point_mapping(&ctx, record)
        .await
        .expect("create");
    assert_eq!(created.source_id, "src-1");

    let list = store
        .list_point_mappings(&ctx, "project-1")
        .await
        .expect("list");
    assert_eq!(list.len(), 1);

    let got = store
        .find_point_mapping(&ctx, "project-1", "src-1")
        .await
        .expect("find");
    assert!(got.is_some());
}
