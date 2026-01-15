use domain::TenantContext;
use ems_storage::{InMemoryProjectStore, ProjectRecord, ProjectStore};

#[tokio::test]
async fn project_belongs_to_tenant() {
    let store = InMemoryProjectStore::with_default_project();
    let ctx = TenantContext::new("tenant-1", "user-1", vec![], vec![], None);
    let matched = store
        .project_belongs_to_tenant(&ctx, "project-1")
        .await
        .expect("query");
    assert!(matched);
}

#[tokio::test]
async fn project_rejects_empty_tenant() {
    let store = InMemoryProjectStore::with_default_project();
    let ctx = TenantContext::default();
    let err = store
        .project_belongs_to_tenant(&ctx, "project-1")
        .await
        .expect_err("tenant required");
    assert_eq!(err.to_string(), "tenant_id required");
}

#[tokio::test]
async fn project_list_includes_created() {
    let store = InMemoryProjectStore::with_default_project();
    let ctx = TenantContext::new("tenant-1", "user-1", vec![], vec![], None);
    let record = ProjectRecord {
        project_id: "project-2".to_string(),
        tenant_id: "tenant-1".to_string(),
        name: "Project 2".to_string(),
        timezone: "UTC".to_string(),
    };
    store.create_project(&ctx, record).await.expect("create");
    let list = store.list_projects(&ctx).await.expect("list");
    assert!(list.iter().any(|item| item.project_id == "project-2"));
}
