use domain::TenantContext;
use ems_auth::JwtManager;

#[test]
fn jwt_issue_and_decode() {
    let jwt = JwtManager::new("secret".to_string(), 3600, 7200);
    let ctx = TenantContext::new(
        "tenant-1",
        "user-1",
        vec!["admin".to_string()],
        vec!["PROJECT.READ".to_string()],
        None,
    );

    let tokens = jwt.issue_tokens(&ctx).expect("tokens");
    let access_ctx = jwt.decode_access(&tokens.access_token).expect("access");
    let refresh_ctx = jwt.decode_refresh(&tokens.refresh_token).expect("refresh");

    assert_eq!(access_ctx.tenant_id, "tenant-1");
    assert_eq!(refresh_ctx.user_id, "user-1");
}
