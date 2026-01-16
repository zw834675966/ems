//! Redis 实时数据写入实现

use crate::error::StorageError;
use crate::online::OnlineStore;
use crate::models::RealtimeRecord;
use crate::traits::RealtimeStore;
use crate::validation::ensure_project_scope;
use domain::{PointValue, PointValueData, TenantContext};
use redis::AsyncCommands;

#[derive(serde::Serialize, serde::Deserialize)]
struct LastValuePayload {
    ts_ms: i64,
    value: String,
    quality: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct OnlinePayload {
    ts_ms: i64,
}

fn last_value_key(value: &PointValue) -> String {
    format!(
        "tenant:{}:project:{}:point:{}:last_value",
        value.tenant_id, value.project_id, value.point_id
    )
}

fn value_to_string(value: &PointValue) -> String {
    match &value.value {
        PointValueData::I64(v) => v.to_string(),
        PointValueData::F64(v) => v.to_string(),
        PointValueData::Bool(v) => v.to_string(),
        PointValueData::String(v) => v.clone(),
    }
}

fn parse_point_id_from_key(key: &str) -> Option<&str> {
    key.split(":point:")
        .nth(1)
        .and_then(|rest| rest.strip_suffix(":last_value"))
}

fn gateway_online_key(tenant_id: &str, project_id: &str, gateway_id: &str) -> String {
    format!(
        "tenant:{}:project:{}:gateway:{}:online",
        tenant_id, project_id, gateway_id
    )
}

fn device_online_key(tenant_id: &str, project_id: &str, device_id: &str) -> String {
    format!(
        "tenant:{}:project:{}:device:{}:online",
        tenant_id, project_id, device_id
    )
}

/// Redis 实时数据存储
pub struct RedisRealtimeStore {
    client: redis::Client,
    last_value_ttl_seconds: Option<u64>,
}

/// Redis Online 状态存储（gateway/device）。
pub struct RedisOnlineStore {
    client: redis::Client,
    ttl_seconds: u64,
}

impl RedisOnlineStore {
    pub fn connect(redis_url: &str, ttl_seconds: u64) -> Result<Self, StorageError> {
        let client =
            redis::Client::open(redis_url).map_err(|err| StorageError::new(err.to_string()))?;
        let ttl = ttl_seconds.max(1);
        Ok(Self {
            client,
            ttl_seconds: ttl,
        })
    }
}

impl RedisRealtimeStore {
    pub fn new(client: redis::Client) -> Self {
        Self {
            client,
            last_value_ttl_seconds: None,
        }
    }

    pub fn new_with_ttl(client: redis::Client, last_value_ttl_seconds: Option<u64>) -> Self {
        Self {
            client,
            last_value_ttl_seconds,
        }
    }

    pub fn connect(redis_url: &str) -> Result<Self, StorageError> {
        let client =
            redis::Client::open(redis_url).map_err(|err| StorageError::new(err.to_string()))?;
        Ok(Self::new(client))
    }

    pub fn connect_with_ttl(
        redis_url: &str,
        last_value_ttl_seconds: Option<u64>,
    ) -> Result<Self, StorageError> {
        let client =
            redis::Client::open(redis_url).map_err(|err| StorageError::new(err.to_string()))?;
        let ttl = match last_value_ttl_seconds {
            Some(value) if value == 0 => None,
            Some(value) => Some(value),
            None => None,
        };
        Ok(Self::new_with_ttl(client, ttl))
    }
}

#[async_trait::async_trait]
impl RealtimeStore for RedisRealtimeStore {
    async fn upsert_last_value(
        &self,
        ctx: &TenantContext,
        value: &PointValue,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, &value.project_id)?;
        if value.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let payload = LastValuePayload {
            ts_ms: value.ts_ms,
            value: value_to_string(value),
            quality: value.quality.clone(),
        };
        let data =
            serde_json::to_string(&payload).map_err(|err| StorageError::new(err.to_string()))?;
        let key = last_value_key(value);
        if let Some(ttl) = self.last_value_ttl_seconds {
            connection
                .set_ex::<_, _, ()>(key, data, ttl)
                .await
                .map_err(|err| StorageError::new(err.to_string()))?;
        } else {
            connection
                .set::<_, _, ()>(key, data)
                .await
                .map_err(|err| StorageError::new(err.to_string()))?;
        }
        Ok(())
    }

    async fn get_last_value(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
    ) -> Result<Option<RealtimeRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let key = format!(
            "tenant:{}:project:{}:point:{}:last_value",
            ctx.tenant_id, project_id, point_id
        );
        let data: Option<String> = connection
            .get(key)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let Some(data) = data else {
            return Ok(None);
        };
        let payload: LastValuePayload =
            serde_json::from_str(&data).map_err(|err| StorageError::new(err.to_string()))?;
        Ok(Some(RealtimeRecord {
            tenant_id: ctx.tenant_id.clone(),
            project_id: project_id.to_string(),
            point_id: point_id.to_string(),
            ts_ms: payload.ts_ms,
            value: payload.value,
            quality: payload.quality,
        }))
    }

    async fn list_last_values(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Vec<RealtimeRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let pattern = format!(
            "tenant:{}:project:{}:point:*:last_value",
            ctx.tenant_id, project_id
        );
        let mut cursor: u64 = 0;
        let mut items = Vec::new();
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut connection)
                .await
                .map_err(|err| StorageError::new(err.to_string()))?;
            for key in keys {
                let point_id = match parse_point_id_from_key(&key) {
                    Some(value) => value.to_string(),
                    None => continue,
                };
                let data: Option<String> = connection
                    .get(&key)
                    .await
                    .map_err(|err| StorageError::new(err.to_string()))?;
                let Some(data) = data else {
                    continue;
                };
                let payload: LastValuePayload = serde_json::from_str(&data)
                    .map_err(|err| StorageError::new(err.to_string()))?;
                items.push(RealtimeRecord {
                    tenant_id: ctx.tenant_id.clone(),
                    project_id: project_id.to_string(),
                    point_id,
                    ts_ms: payload.ts_ms,
                    value: payload.value,
                    quality: payload.quality,
                });
            }
            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }
        Ok(items)
    }
}

#[async_trait::async_trait]
impl OnlineStore for RedisOnlineStore {
    async fn touch_gateway(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
        ts_ms: i64,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let payload = OnlinePayload { ts_ms };
        let data =
            serde_json::to_string(&payload).map_err(|err| StorageError::new(err.to_string()))?;
        let key = gateway_online_key(&ctx.tenant_id, project_id, gateway_id);
        connection
            .set_ex::<_, _, ()>(key, data, self.ttl_seconds)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        Ok(())
    }

    async fn touch_device(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
        ts_ms: i64,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let payload = OnlinePayload { ts_ms };
        let data =
            serde_json::to_string(&payload).map_err(|err| StorageError::new(err.to_string()))?;
        let key = device_online_key(&ctx.tenant_id, project_id, device_id);
        connection
            .set_ex::<_, _, ()>(key, data, self.ttl_seconds)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        Ok(())
    }

    async fn get_gateway_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_id: &str,
    ) -> Result<Option<i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let key = gateway_online_key(&ctx.tenant_id, project_id, gateway_id);
        let data: Option<String> = connection
            .get(key)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let Some(data) = data else {
            return Ok(None);
        };
        let payload: OnlinePayload =
            serde_json::from_str(&data).map_err(|err| StorageError::new(err.to_string()))?;
        Ok(Some(payload.ts_ms))
    }

    async fn get_device_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let key = device_online_key(&ctx.tenant_id, project_id, device_id);
        let data: Option<String> = connection
            .get(key)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let Some(data) = data else {
            return Ok(None);
        };
        let payload: OnlinePayload =
            serde_json::from_str(&data).map_err(|err| StorageError::new(err.to_string()))?;
        Ok(Some(payload.ts_ms))
    }

    async fn list_gateways_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        gateway_ids: &[String],
    ) -> Result<std::collections::HashMap<String, i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        if gateway_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let keys: Vec<String> = gateway_ids
            .iter()
            .map(|id| gateway_online_key(&ctx.tenant_id, project_id, id))
            .collect();
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let values: Vec<Option<String>> = connection
            .mget(keys)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let mut result = std::collections::HashMap::new();
        for (id, value) in gateway_ids.iter().zip(values.into_iter()) {
            let Some(value) = value else { continue };
            let payload: OnlinePayload = match serde_json::from_str(&value) {
                Ok(payload) => payload,
                Err(_) => continue,
            };
            result.insert(id.clone(), payload.ts_ms);
        }
        Ok(result)
    }

    async fn list_devices_last_seen_at_ms(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        device_ids: &[String],
    ) -> Result<std::collections::HashMap<String, i64>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        if device_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let keys: Vec<String> = device_ids
            .iter()
            .map(|id| device_online_key(&ctx.tenant_id, project_id, id))
            .collect();
        let mut connection = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let values: Vec<Option<String>> = connection
            .mget(keys)
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let mut result = std::collections::HashMap::new();
        for (id, value) in device_ids.iter().zip(values.into_iter()) {
            let Some(value) = value else { continue };
            let payload: OnlinePayload = match serde_json::from_str(&value) {
                Ok(payload) => payload,
                Err(_) => continue,
            };
            result.insert(id.clone(), payload.ts_ms);
        }
        Ok(result)
    }
}
