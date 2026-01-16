use std::sync::Arc;

use ems_auth::{AuthError, AuthService, JwtManager};
use ems_storage::InMemoryUserStore;

#[tokio::test]
async fn refresh_token_is_single_use_after_rotation() {
    let user_store: Arc<InMemoryUserStore> = Arc::new(InMemoryUserStore::with_default_admin());
    let jwt = JwtManager::new("secret".to_string(), 3600, 7200);
    let auth = AuthService::new(user_store, jwt);

    let (_, tokens1) = auth.login("admin", "admin123").await.expect("login");
    let tokens2 = auth
        .refresh(&tokens1.refresh_token)
        .await
        .expect("refresh");
    assert_ne!(tokens1.refresh_token, tokens2.refresh_token);

    let result = auth.refresh(&tokens1.refresh_token).await;
    assert!(matches!(result, Err(AuthError::TokenInvalid)));
}
