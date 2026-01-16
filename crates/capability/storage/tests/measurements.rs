use domain::{PointValue, PointValueData, TenantContext};
use ems_storage::{
    InMemoryMeasurementStore, MeasurementAggFn, MeasurementAggregation, MeasurementStore,
    MeasurementsQueryOptions, TimeOrder,
};

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
        quality: None,
    }
}

#[tokio::test]
async fn measurements_list_filters_by_range() {
    let store = InMemoryMeasurementStore::new();
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec![],
        vec![],
        Some("project-1".to_string()),
    );
    let values = vec![
        sample_value(
            "tenant-1",
            "project-1",
            "point-1",
            1000,
            PointValueData::I64(1),
        ),
        sample_value(
            "tenant-1",
            "project-1",
            "point-1",
            2000,
            PointValueData::I64(2),
        ),
        sample_value(
            "tenant-1",
            "project-1",
            "point-1",
            3000,
            PointValueData::I64(3),
        ),
    ];
    store
        .write_measurements(&ctx, &values)
        .await
        .expect("write");

    let list = store
        .list_measurements(&ctx, "project-1", "point-1", Some(1500), Some(2500), 10)
        .await
        .expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].ts_ms, 2000);
    assert_eq!(list[0].value, "2");
}

#[tokio::test]
async fn measurements_list_respects_limit() {
    let store = InMemoryMeasurementStore::new();
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec![],
        vec![],
        Some("project-1".to_string()),
    );
    let values = vec![
        sample_value(
            "tenant-1",
            "project-1",
            "point-1",
            1000,
            PointValueData::I64(1),
        ),
        sample_value(
            "tenant-1",
            "project-1",
            "point-1",
            2000,
            PointValueData::I64(2),
        ),
        sample_value(
            "tenant-1",
            "project-1",
            "point-1",
            3000,
            PointValueData::I64(3),
        ),
    ];
    store
        .write_measurements(&ctx, &values)
        .await
        .expect("write");

    let list = store
        .list_measurements(&ctx, "project-1", "point-1", None, None, 2)
        .await
        .expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].ts_ms, 1000);
    assert_eq!(list[1].ts_ms, 2000);
}

#[tokio::test]
async fn measurements_reject_tenant_mismatch() {
    let store = InMemoryMeasurementStore::new();
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec![],
        vec![],
        Some("project-1".to_string()),
    );
    let value = sample_value(
        "tenant-2",
        "project-1",
        "point-1",
        1000,
        PointValueData::I64(1),
    );
    let err = store
        .write_measurement(&ctx, &value)
        .await
        .expect_err("tenant mismatch");
    assert_eq!(err.to_string(), "tenant mismatch");
}

#[tokio::test]
async fn measurements_support_cursor_and_order() {
    let store = InMemoryMeasurementStore::new();
    let ctx = TenantContext::new(
        "tenant-1".to_string(),
        "user-1".to_string(),
        Vec::new(),
        Vec::new(),
        Some("project-1".to_string()),
    );

    let values = vec![
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 1000,
            value: PointValueData::F64(1.0),
            quality: None,
        },
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 2000,
            value: PointValueData::F64(2.0),
            quality: None,
        },
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 3000,
            value: PointValueData::F64(3.0),
            quality: None,
        },
    ];
    store
        .write_measurements(&ctx, &values)
        .await
        .expect("write measurements");

    let items = store
        .query_measurements(
            &ctx,
            "project-1",
            "point-1",
            MeasurementsQueryOptions {
                from_ms: None,
                to_ms: None,
                cursor_ts_ms: Some(1500),
                order: TimeOrder::Asc,
                limit: 10,
                aggregation: None,
            },
        )
        .await
        .expect("query measurements");
    assert_eq!(items.iter().map(|i| i.ts_ms).collect::<Vec<_>>(), vec![2000, 3000]);

    let items = store
        .query_measurements(
            &ctx,
            "project-1",
            "point-1",
            MeasurementsQueryOptions {
                from_ms: None,
                to_ms: None,
                cursor_ts_ms: Some(2500),
                order: TimeOrder::Desc,
                limit: 10,
                aggregation: None,
            },
        )
        .await
        .expect("query measurements");
    assert_eq!(items.iter().map(|i| i.ts_ms).collect::<Vec<_>>(), vec![2000, 1000]);
}

#[tokio::test]
async fn measurements_support_aggregation() {
    let store = InMemoryMeasurementStore::new();
    let ctx = TenantContext::new(
        "tenant-1".to_string(),
        "user-1".to_string(),
        Vec::new(),
        Vec::new(),
        Some("project-1".to_string()),
    );

    let values = vec![
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 1100,
            value: PointValueData::F64(1.0),
            quality: None,
        },
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 1900,
            value: PointValueData::F64(3.0),
            quality: None,
        },
        PointValue {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            point_id: "point-1".to_string(),
            ts_ms: 2100,
            value: PointValueData::F64(5.0),
            quality: None,
        },
    ];
    store
        .write_measurements(&ctx, &values)
        .await
        .expect("write measurements");

    let items = store
        .query_measurements(
            &ctx,
            "project-1",
            "point-1",
            MeasurementsQueryOptions {
                from_ms: None,
                to_ms: None,
                cursor_ts_ms: None,
                order: TimeOrder::Asc,
                limit: 10,
                aggregation: Some(MeasurementAggregation {
                    bucket_ms: 1000,
                    func: MeasurementAggFn::Avg,
                }),
            },
        )
        .await
        .expect("query measurements");
    assert_eq!(items.iter().map(|i| i.ts_ms).collect::<Vec<_>>(), vec![1000, 2000]);
    assert_eq!(items[0].value.parse::<f64>().ok(), Some(2.0));
}
