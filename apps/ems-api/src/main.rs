//! EMS API 主入口

mod handlers;
mod middleware;
mod routes;
mod utils;

use axum::{Router, middleware as axum_middleware};
use ems_auth::{AuthService, JwtManager};
use ems_config::AppConfig;
use ems_storage::{
    PgDeviceStore, PgGatewayStore, PgPointMappingStore, PgPointStore, PgProjectStore, PgUserStore,
    connect_pool,
};
use ems_telemetry::init_tracing;
use std::{env, path::PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WebAdminMode {
    Off,
    On,
    Only,
}

impl WebAdminMode {
    fn from_env() -> Self {
        match env::var("EMS_WEB_ADMIN")
            .unwrap_or_else(|_| "off".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "1" | "true" | "on" => Self::On,
            "only" => Self::Only,
            _ => Self::Off,
        }
    }
}

fn spawn_web_admin() -> Result<tokio::process::Child, std::io::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let web_admin_dir = manifest_dir.join("../..").join("web/admin");
    if !web_admin_dir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("web/admin not found at {:?}", web_admin_dir),
        ));
    }
    Command::new("pnpm").arg("dev").current_dir(web_admin_dir).spawn()
}

#[derive(Clone)]
struct AppState {
    auth: Arc<AuthService>,
    project_store: Arc<dyn ems_storage::ProjectStore>,
    gateway_store: Arc<dyn ems_storage::GatewayStore>,
    device_store: Arc<dyn ems_storage::DeviceStore>,
    point_store: Arc<dyn ems_storage::PointStore>,
    point_mapping_store: Arc<dyn ems_storage::PointMappingStore>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    init_tracing();

    let web_admin_mode = WebAdminMode::from_env();
    let mut web_admin_child = None;
    if web_admin_mode != WebAdminMode::Off {
        match spawn_web_admin() {
            Ok(child) => {
                info!("web/admin started via pnpm dev");
                web_admin_child = Some(child);
            }
            Err(err) => {
                warn!("failed to start web/admin: {}", err);
                if web_admin_mode == WebAdminMode::Only {
                    return Err(err.into());
                }
            }
        }
    }
    if web_admin_mode == WebAdminMode::Only {
        if let Some(mut child) = web_admin_child {
            let _ = child.wait().await?;
        }
        return Ok(());
    }
    if let Some(mut child) = web_admin_child {
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => info!("web/admin exited: {}", status),
                Err(err) => warn!("web/admin wait failed: {}", err),
            }
        });
    }

    let pool = connect_pool(&config.database_url).await?;
    let user_store = Arc::new(PgUserStore::new(pool.clone()));
    let jwt = JwtManager::new(
        config.jwt_secret.clone(),
        config.jwt_access_ttl_seconds,
        config.jwt_refresh_ttl_seconds,
    );
    let auth = Arc::new(AuthService::new(user_store, jwt));

    let project_store: Arc<dyn ems_storage::ProjectStore> =
        Arc::new(PgProjectStore::new(pool.clone()));
    let gateway_store: Arc<dyn ems_storage::GatewayStore> =
        Arc::new(PgGatewayStore::new(pool.clone()));
    let device_store: Arc<dyn ems_storage::DeviceStore> =
        Arc::new(PgDeviceStore::new(pool.clone()));
    let point_store: Arc<dyn ems_storage::PointStore> = Arc::new(PgPointStore::new(pool.clone()));
    let point_mapping_store: Arc<dyn ems_storage::PointMappingStore> =
        Arc::new(PgPointMappingStore::new(pool));

    let state = AppState {
        auth,
        project_store,
        gateway_store,
        device_store,
        point_store,
        point_mapping_store,
    };

    let api = routes::create_api_router();
    let app = Router::new()
        .merge(api.clone())
        .nest("/api", api)
        .with_state(state)
        .layer(axum_middleware::from_fn(middleware::request_context));

    let listener = tokio::net::TcpListener::bind(&config.http_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
