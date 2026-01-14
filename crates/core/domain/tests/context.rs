use domain::TenantContext;

#[test]
fn tenant_context_builds() {
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec!["admin".to_string()],
        vec!["PROJECT.READ".to_string()],
        None,
    );

    assert_eq!(ctx.tenant_id, "tenant-1");
    assert_eq!(ctx.user_id, "user-1");
    assert_eq!(ctx.roles.len(), 1);
    assert_eq!(ctx.permissions.len(), 1);
    assert!(ctx.project_scope.is_none());
}
