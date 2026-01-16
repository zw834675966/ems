//! 时序写入内存实现
//!
//! 仅用于本地测试和占位。

use crate::error::StorageError;
use crate::models::MeasurementRecord;
use crate::traits::{
    MeasurementAggFn, MeasurementAggregation, MeasurementStore, MeasurementsQueryOptions, TimeOrder,
};
use crate::validation::ensure_project_scope;
use domain::{PointValue, PointValueData, TenantContext};
use std::sync::RwLock;

/// 时序写入内存存储
pub struct InMemoryMeasurementStore {
    values: RwLock<Vec<PointValue>>,
}

impl InMemoryMeasurementStore {
    /// 创建新的时序写入存储
    pub fn new() -> Self {
        Self {
            values: RwLock::new(Vec::new()),
        }
    }

    /// 获取当前累计的测点值数量（用于测试）
    pub fn len(&self) -> usize {
        self.values.read().map(|v| v.len()).unwrap_or(0)
    }
}

fn value_to_string(value: &PointValue) -> String {
    match &value.value {
        PointValueData::I64(v) => v.to_string(),
        PointValueData::F64(v) => v.to_string(),
        PointValueData::Bool(v) => v.to_string(),
        PointValueData::String(v) => v.clone(),
    }
}

#[async_trait::async_trait]
impl MeasurementStore for InMemoryMeasurementStore {
    async fn write_measurement(
        &self,
        ctx: &TenantContext,
        value: &PointValue,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, &value.project_id)?;
        if value.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut values = self
            .values
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        values.push(value.clone());
        Ok(())
    }

    async fn write_measurements(
        &self,
        ctx: &TenantContext,
        values: &[PointValue],
    ) -> Result<usize, StorageError> {
        for value in values {
            ensure_project_scope(ctx, &value.project_id)?;
            if value.tenant_id != ctx.tenant_id {
                return Err(StorageError::new("tenant mismatch"));
            }
        }
        let mut store = self
            .values
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        store.extend(values.iter().cloned());
        Ok(values.len())
    }

    async fn query_measurements(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        point_id: &str,
        options: MeasurementsQueryOptions,
    ) -> Result<Vec<MeasurementRecord>, StorageError> {
        ensure_project_scope(ctx, project_id)?;
        let limit = options.limit.max(0) as usize;
        let values = self
            .values
            .read()
            .map_err(|_| StorageError::new("lock failed"))?;
        let mut selected = Vec::new();
        for value in values.iter() {
            if value.tenant_id != ctx.tenant_id
                || value.project_id != project_id
                || value.point_id != point_id
            {
                continue;
            }
            if let Some(from) = options.from_ms {
                if value.ts_ms < from {
                    continue;
                }
            }
            if let Some(to) = options.to_ms {
                if value.ts_ms > to {
                    continue;
                }
            }
            selected.push(value.clone());
        }

        selected.sort_by_key(|item| item.ts_ms);

        if let Some(aggregation) = options.aggregation {
            return Ok(aggregate_values(
                &selected,
                aggregation,
                limit,
                ctx,
                project_id,
                point_id,
                options.order,
                options.cursor_ts_ms,
            ));
        }

        if let Some(cursor_ts_ms) = options.cursor_ts_ms {
            selected.retain(|item| match options.order {
                TimeOrder::Asc => item.ts_ms > cursor_ts_ms,
                TimeOrder::Desc => item.ts_ms < cursor_ts_ms,
            });
        }

        if matches!(options.order, TimeOrder::Desc) {
            selected.reverse();
        }

        let mut items = Vec::new();
        for value in selected.iter() {
            items.push(MeasurementRecord {
                tenant_id: value.tenant_id.clone(),
                project_id: value.project_id.clone(),
                point_id: value.point_id.clone(),
                ts_ms: value.ts_ms,
                value: value_to_string(value),
                quality: value.quality.clone(),
            });
            if limit > 0 && items.len() >= limit {
                break;
            }
        }
        Ok(items)
    }
}

fn numeric_value(value: &PointValue) -> Option<f64> {
    match &value.value {
        PointValueData::I64(v) => Some(*v as f64),
        PointValueData::F64(v) => Some(*v),
        PointValueData::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
        PointValueData::String(v) => v.parse::<f64>().ok(),
    }
}

fn aggregate_values(
    values: &[PointValue],
    aggregation: MeasurementAggregation,
    limit: usize,
    ctx: &TenantContext,
    project_id: &str,
    point_id: &str,
    order: TimeOrder,
    cursor_ts_ms: Option<i64>,
) -> Vec<MeasurementRecord> {
    if aggregation.bucket_ms <= 0 {
        return Vec::new();
    }
    let bucket_ms = aggregation.bucket_ms;
    let mut buckets: std::collections::BTreeMap<i64, Vec<&PointValue>> =
        std::collections::BTreeMap::new();
    for value in values {
        let bucket_start = value.ts_ms.div_euclid(bucket_ms) * bucket_ms;
        buckets.entry(bucket_start).or_default().push(value);
    }

    let mut items = Vec::new();
    let iter: Box<dyn Iterator<Item = (&i64, &Vec<&PointValue>)>> = match order {
        TimeOrder::Asc => Box::new(buckets.iter()),
        TimeOrder::Desc => Box::new(buckets.iter().rev()),
    };

    for (bucket_start, bucket_values) in iter {
        if let Some(cursor) = cursor_ts_ms {
            match order {
                TimeOrder::Asc if *bucket_start <= cursor => continue,
                TimeOrder::Desc if *bucket_start >= cursor => continue,
                _ => {}
            }
        }

        let value = match aggregation.func {
            MeasurementAggFn::Count => Some(bucket_values.len() as f64),
            MeasurementAggFn::Sum => bucket_values
                .iter()
                .filter_map(|item| numeric_value(item))
                .reduce(|acc, item| acc + item),
            MeasurementAggFn::Avg => {
                let mut count = 0u64;
                let sum = bucket_values
                    .iter()
                    .filter_map(|item| numeric_value(item))
                    .fold(0.0, |acc, item| {
                        count += 1;
                        acc + item
                    });
                if count == 0 {
                    None
                } else {
                    Some(sum / count as f64)
                }
            }
            MeasurementAggFn::Min => bucket_values
                .iter()
                .filter_map(|item| numeric_value(item))
                .reduce(|acc, item| acc.min(item)),
            MeasurementAggFn::Max => bucket_values
                .iter()
                .filter_map(|item| numeric_value(item))
                .reduce(|acc, item| acc.max(item)),
        };

        let Some(value) = value else { continue };
        let value_str = match aggregation.func {
            MeasurementAggFn::Count => format!("{}", value as i64),
            _ => value.to_string(),
        };
        items.push(MeasurementRecord {
            tenant_id: ctx.tenant_id.clone(),
            project_id: project_id.to_string(),
            point_id: point_id.to_string(),
            ts_ms: *bucket_start,
            value: value_str,
            quality: None,
        });
        if limit > 0 && items.len() >= limit {
            break;
        }
    }

    items
}
