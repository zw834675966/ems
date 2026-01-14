use ems_storage::{InMemoryUserStore, UserStore};
use domain::TenantContext;

#[tokio::test]
async fn find_default_admin() {
    let store = InMemoryUserStore::with_default_admin();
    let ctx = TenantContext::default();
    let user = store
        .find_by_username(&ctx, "admin")
        .await
        .expect("query")
        .expect("admin");
    assert_eq!(user.username, "admin");
    assert_eq!(user.tenant_id, "tenant-1");
}
