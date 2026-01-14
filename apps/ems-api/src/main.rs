//! M0 最小 HTTP API（登录/刷新/动态路由）与请求追踪 ID。

use api_contract::{
    ApiResponse, AsyncRoute, LoginRequest, LoginResponse, RefreshTokenRequest,
    RefreshTokenResponse, RouteMeta,
};
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use ems_auth::{AuthError, AuthService, JwtManager};
use ems_config::AppConfig;
use ems_storage::PgUserStore;
use ems_telemetry::{init_tracing, new_request_ids};
use std::sync::Arc;
use tracing::Instrument;

#[derive(Clone)]
struct AppState {
    auth: Arc<AuthService>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载本地 .env（如存在），便于直接 cargo run 启动
    dotenvy::dotenv().ok();
    // 从环境变量加载运行配置
    let config = AppConfig::from_env()?;
    // 初始化结构化日志
    init_tracing();

    // Postgres 用户存储（需先执行 migrations/seed）
    let user_store = Arc::new(PgUserStore::connect(&config.database_url).await?);
    // JWT 管理器
    let jwt = JwtManager::new(
        config.jwt_secret.clone(),
        config.jwt_access_ttl_seconds,
        config.jwt_refresh_ttl_seconds,
    );
    let auth = Arc::new(AuthService::new(user_store, jwt));
    let state = AppState { auth };

    // M0 路由：健康检查、登录、刷新、动态路由
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/login", post(login))
        .route("/login", post(login))
        .route("/api/refresh-token", post(refresh_token))
        .route("/refresh-token", post(refresh_token))
        .route("/api/get-async-routes", get(get_async_routes))
        .route("/get-async-routes", get(get_async_routes))
        .with_state(state)
        // 注入 request_id/trace_id
        .layer(middleware::from_fn(request_context));

    let listener = tokio::net::TcpListener::bind(&config.http_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Response {
    match state.auth.login(&req.username, &req.password).await {
        Ok((user, tokens)) => {
            let response = LoginResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires: tokens.expires_at.saturating_mul(1000),
                username: user.username.clone(),
                nickname: user.username,
                avatar: "".to_string(),
                roles: user.roles,
                permissions: user.permissions,
            };
            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        Err(AuthError::InvalidCredentials) => auth_error(StatusCode::UNAUTHORIZED),
        Err(err) => internal_error(err),
    }
}

async fn refresh_token(
    State(state): State<AppState>,
    Json(req): Json<RefreshTokenRequest>,
) -> Response {
    match state.auth.refresh(&req.refresh_token) {
        Ok(tokens) => {
            let response = RefreshTokenResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expires: tokens.expires_at.saturating_mul(1000),
            };
            (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
        }
        Err(AuthError::TokenInvalid | AuthError::TokenExpired) => {
            auth_error(StatusCode::UNAUTHORIZED)
        }
        Err(err) => internal_error(err),
    }
}

async fn get_async_routes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    // 校验 access token
    let token = match bearer_token(&headers) {
        Some(token) => token,
        None => return auth_error(StatusCode::UNAUTHORIZED),
    };

    if let Err(AuthError::TokenInvalid | AuthError::TokenExpired) =
        state.auth.verify_access_token(token)
    {
        return auth_error(StatusCode::UNAUTHORIZED);
    }

    // 最小动态路由响应（可按前端需要扩展）
    let routes = vec![AsyncRoute {
        path: "/ems".to_string(),
        name: "EMS".to_string(),
        component: "Layout".to_string(),
        meta: RouteMeta {
            title: "EMS".to_string(),
            icon: "monitor".to_string(),
            rank: 1,
            roles: None,
            auths: None,
        },
        children: Vec::new(),
    }];

    (StatusCode::OK, Json(ApiResponse::success(routes))).into_response()
}

async fn request_context(mut req: Request<Body>, next: Next) -> Response {
    // 生成 request_id 与 trace_id，并注入请求扩展与日志
    let ids = new_request_ids();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    req.extensions_mut().insert(ids.clone());

    let span = tracing::info_span!(
        "request",
        request_id = %ids.request_id,
        trace_id = %ids.trace_id,
        method = %method,
        path = %path
    );

    let mut response = next.run(req).instrument(span).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&ids.request_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    response.headers_mut().insert(
        "x-trace-id",
        HeaderValue::from_str(&ids.trace_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    response
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let header_value = headers.get(header::AUTHORIZATION)?;
    let auth_str = header_value.to_str().ok()?;
    auth_str.strip_prefix("Bearer ")
}

fn auth_error(status: StatusCode) -> Response {
    (
        status,
        Json(ApiResponse::<()>::error(
            "AUTH.UNAUTHORIZED",
            "unauthorized",
        )),
    )
        .into_response()
}

fn internal_error(err: AuthError) -> Response {
    let message = err.to_string();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::<()>::error("INTERNAL.ERROR", message)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::bearer_token;
    use axum::http::{header, HeaderMap, HeaderValue};

    #[test]
    fn bearer_token_extracts() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token-1"),
        );
        assert_eq!(bearer_token(&headers), Some("token-1"));
    }
}
