//! 内存存储实现模块
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 包含以下实现：
//! - UserStore: InMemoryUserStore
//! - ProjectStore: InMemoryProjectStore
//! - GatewayStore: InMemoryGatewayStore
//! - DeviceStore: InMemoryDeviceStore
//! - PointStore: InMemoryPointStore
//! - PointMappingStore: InMemoryPointMappingStore
//! - CollectionStrategyStore: InMemoryCollectionStrategyStore

#![allow(clippy::new_without_default)]
#![allow(clippy::len_without_is_empty)]

pub mod audit;
pub mod collection_strategy;
pub mod command;
pub mod command_receipt;
pub mod device;
pub mod gateway;
pub mod measurement;
pub mod online;
pub mod point;
pub mod point_mapping;
pub mod project;
pub mod realtime;
pub mod system_logs;
pub mod user;

pub use audit::*;
pub use collection_strategy::*;
pub use command::*;
pub use command_receipt::*;
pub use device::*;
pub use gateway::*;
pub use measurement::*;
pub use online::*;
pub use point::*;
pub use point_mapping::*;
pub use project::*;
pub use realtime::*;
pub use system_logs::*;
pub use user::*;
