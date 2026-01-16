use domain::TenantContext;
use ems_storage::{InMemoryOnlineStore, OnlineStore};

#[tokio::test]
async fn online_touch_and_get() {
    let store = InMemoryOnlineStore::new();
    let ctx = TenantContext::new(
        "tenant-1".to_string(),
        "user-1".to_string(),
        Vec::new(),
        Vec::new(),
        Some("project-1".to_string()),
    );

    store
        .touch_gateway(&ctx, "project-1", "gateway-1", 1234)
        .await
        .expect("touch gateway");
    store
        .touch_device(&ctx, "project-1", "device-1", 2345)
        .await
        .expect("touch device");

    let gw = store
        .get_gateway_last_seen_at_ms(&ctx, "project-1", "gateway-1")
        .await
        .expect("get gateway");
    assert_eq!(gw, Some(1234));

    let dev = store
        .get_device_last_seen_at_ms(&ctx, "project-1", "device-1")
        .await
        .expect("get device");
    assert_eq!(dev, Some(2345));
}

#[tokio::test]
async fn online_list_batch() {
    let store = InMemoryOnlineStore::new();
    let ctx = TenantContext::new(
        "tenant-1".to_string(),
        "user-1".to_string(),
        Vec::new(),
        Vec::new(),
        Some("project-1".to_string()),
    );

    store
        .touch_gateway(&ctx, "project-1", "gateway-1", 1000)
        .await
        .expect("touch gateway");
    store
        .touch_gateway(&ctx, "project-1", "gateway-2", 2000)
        .await
        .expect("touch gateway");

    let gateways = store
        .list_gateways_last_seen_at_ms(
            &ctx,
            "project-1",
            &["gateway-1".to_string(), "gateway-3".to_string()],
        )
        .await
        .expect("list gateways");
    assert_eq!(gateways.get("gateway-1").copied(), Some(1000));
    assert!(gateways.get("gateway-3").is_none());
}

