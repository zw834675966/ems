//! # EMS Collector 采集服务
//!
//! 负责从外部数据源（MQTT, Modbus/TCP 等）采集遥测数据，并将标准化后的数据持久化。
//! 该服务作为一个独立进程运行，与 API 服务解耦。

use ems_config::AppConfig;
use ems_telemetry::init_tracing;
use std::sync::Arc;
use tracing::{info, warn};

mod ingest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载 .env 环境变量
    dotenvy::dotenv().ok();

    // 2. 从环境变量读取应用配置
    let config = AppConfig::from_env()?;

    // 3. 初始化 tracing 日志系统
    init_tracing(config.log_format == "json");

    info!("🚀 EMS Collector 服务启动中...");

    // 4. 建立 PostgreSQL 数据库连接池
    let pool = ems_storage::connect_pool(&config.database_url).await?;

    // 5. 初始化存储层实例
    let gateway_store = Arc::new(ems_storage::PgGatewayStore::new(pool.clone()));
    let point_store = Arc::new(ems_storage::PgPointStore::new(pool.clone()));
    let device_store = Arc::new(ems_storage::PgDeviceStore::new(pool.clone()));
    let point_mapping_store = Arc::new(ems_storage::PgPointMappingStore::new(pool.clone()));
    let measurement_store = Arc::new(ems_storage::PgMeasurementStore::new(pool.clone()));
    let collection_strategy_store = Arc::new(ems_storage::PgCollectionStrategyStore::new(pool.clone()));

    // 实时数据与在线状态缓存（内存实现）
    let realtime_store = Arc::new(ems_storage::InMemoryRealtimeStore::new());
    let online_store = Arc::new(ems_storage::InMemoryOnlineStore::new());

    // 6. 初始化 WAL 存储
    let wal_store = Arc::new(ems_storage::InMemoryIngestWalStore::new());

    // 7. 启动采集任务
    info!("正在启动采集链路...");
    let _ingest_handle = ingest::spawn_ingest(
        &config,
        gateway_store,
        point_mapping_store,
        point_store,
        device_store,
        measurement_store,
        realtime_store,
        online_store,
        collection_strategy_store,
        wal_store,
    );

    info!("🚀 EMS Collector 服务已就绪");

    // 保持进程运行
    tokio::signal::ctrl_c().await?;
    info!("👋 EMS Collector 服务正在停止...");

    Ok(())
}
