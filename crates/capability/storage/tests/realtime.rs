use domain::{PointValue, PointValueData, TenantContext};
use ems_storage::{InMemoryRealtimeStore, RealtimeStore};

fn sample_value(
    tenant_id: &str,
    project_id: &str,
    point_id: &str,
    ts_ms: i64,
    value: PointValueData,
) -> PointValue {
    PointValue {
        tenant_id: tenant_id.to_string(),
        project_id: project_id.to_string(),
        point_id: point_id.to_string(),
        ts_ms,
        value,
        quality: Some("good".to_string()),
    }
}

#[tokio::test]
async fn realtime_upsert_and_get() {
    let store = InMemoryRealtimeStore::new();
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec![],
        vec![],
        Some("project-1".to_string()),
    );
    let value = sample_value(
        "tenant-1",
        "project-1",
        "point-1",
        1000,
        PointValueData::F64(12.3),
    );
    store.upsert_last_value(&ctx, &value).await.expect("write");

    let record = store
        .get_last_value(&ctx, "project-1", "point-1")
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.ts_ms, 1000);
    assert_eq!(record.value, "12.3");
    assert_eq!(record.quality.as_deref(), Some("good"));

    let list = store
        .list_last_values(&ctx, "project-1")
        .await
        .expect("list");
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn realtime_list_filters_project() {
    let store = InMemoryRealtimeStore::new();
    let ctx = TenantContext::new("tenant-1", "user-1", vec![], vec![], None);
    let value_1 = sample_value(
        "tenant-1",
        "project-1",
        "point-1",
        1000,
        PointValueData::I64(1),
    );
    let value_2 = sample_value(
        "tenant-1",
        "project-2",
        "point-2",
        2000,
        PointValueData::I64(2),
    );
    store
        .upsert_last_value(&ctx, &value_1)
        .await
        .expect("write");
    store
        .upsert_last_value(&ctx, &value_2)
        .await
        .expect("write");

    let list = store
        .list_last_values(&ctx, "project-1")
        .await
        .expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].point_id, "point-1");
}

#[tokio::test]
async fn realtime_rejects_project_scope_mismatch() {
    let store = InMemoryRealtimeStore::new();
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec![],
        vec![],
        Some("project-1".to_string()),
    );
    let err = store
        .get_last_value(&ctx, "project-2", "point-1")
        .await
        .expect_err("scope mismatch");
    assert_eq!(err.to_string(), "project scope mismatch");
}
