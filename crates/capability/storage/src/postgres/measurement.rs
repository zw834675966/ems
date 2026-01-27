//! Postgres 时序写入实现

use crate::error::StorageError;
use crate::models::MeasurementRecord;
use crate::traits::{MeasurementAggFn, MeasurementStore, MeasurementsQueryOptions, TimeOrder};
use crate::validation::ensure_project_scope;
use domain::{PointValue, PointValueData, TenantContext};
use sqlx::{PgPool, Row};

pub struct PgMeasurementStore {
    pub pool: PgPool,
}

impl PgMeasurementStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = crate::connection::connect_pool(database_url).await?;
        Ok(Self { pool })
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
impl MeasurementStore for PgMeasurementStore {
    async fn write_measurement(
        &self,
        ctx: &TenantContext,
        value: &PointValue,
    ) -> Result<(), StorageError> {
        ensure_project_scope(ctx, &value.project_id)?;
        if value.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let value_str = value_to_string(value);
        sqlx::query(
            "insert into measurement (tenant_id, project_id, point_id, ts, value, quality) \
             values ($1, $2, $3, to_timestamp($4 / 1000.0), $5, $6)",
        )
        .bind(&value.tenant_id)
        .bind(&value.project_id)
        .bind(&value.point_id)
        .bind(value.ts_ms as f64)
        .bind(value_str)
        .bind(&value.quality)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn write_measurements(
        &self,
        ctx: &TenantContext,
        values: &[PointValue],
    ) -> Result<usize, StorageError> {
        if values.is_empty() {
            return Ok(0);
        }

        let mut tenant_ids = Vec::with_capacity(values.len());
        let mut project_ids = Vec::with_capacity(values.len());
        let mut point_ids = Vec::with_capacity(values.len());
        let mut ts_mss = Vec::with_capacity(values.len());
        let mut values_str = Vec::with_capacity(values.len());
        let mut qualities = Vec::with_capacity(values.len());

        for value in values {
            ensure_project_scope(ctx, &value.project_id)?;
            if value.tenant_id != ctx.tenant_id {
                return Err(StorageError::new("tenant mismatch"));
            }
            tenant_ids.push(value.tenant_id.clone());
            project_ids.push(value.project_id.clone());
            point_ids.push(value.point_id.clone());
            ts_mss.push(value.ts_ms as f64);
            values_str.push(value_to_string(value));
            qualities.push(value.quality.clone());
        }

        sqlx::query(
            "insert into measurement (tenant_id, project_id, point_id, ts, value, quality) \
             select u.tenant_id, u.project_id, u.point_id, to_timestamp(u.ts_ms / 1000.0), u.value, u.quality \
             from unnest($1, $2, $3, $4, $5, $6) as u(tenant_id, project_id, point_id, ts_ms, value, quality)",
        )
        .bind(&tenant_ids)
        .bind(&project_ids)
        .bind(&point_ids)
        .bind(&ts_mss)
        .bind(&values_str)
        .bind(&qualities)
        .execute(&self.pool)
        .await?;

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
        let limit = options.limit.max(0);
        if limit == 0 {
            return Ok(Vec::new());
        }

        if let Some(aggregation) = options.aggregation {
            return query_measurements_aggregated(
                self,
                ctx,
                project_id,
                point_id,
                options,
                aggregation,
            )
            .await;
        }

        query_measurements_raw(self, ctx, project_id, point_id, options).await
    }
}

async fn query_measurements_raw(
    store: &PgMeasurementStore,
    ctx: &TenantContext,
    project_id: &str,
    point_id: &str,
    options: MeasurementsQueryOptions,
) -> Result<Vec<MeasurementRecord>, StorageError> {
    let (cursor_op, order_by) = match options.order {
        TimeOrder::Asc => (">", "asc"),
        TimeOrder::Desc => ("<", "desc"),
    };
    let sql = format!(
        "select tenant_id, project_id, point_id, \
         (extract(epoch from ts) * 1000)::bigint as ts_ms, \
         value, quality \
         from measurement \
         where tenant_id = $1 \
         and project_id = $2 \
         and point_id = $3 \
         and ($4 is null or ts >= to_timestamp($4 / 1000.0)) \
         and ($5 is null or ts <= to_timestamp($5 / 1000.0)) \
         and ($6 is null or ts {cursor_op} to_timestamp($6 / 1000.0)) \
         order by ts {order_by} \
         limit $7"
    );

    let rows = sqlx::query(&sql)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .bind(options.from_ms)
        .bind(options.to_ms)
        .bind(options.cursor_ts_ms)
        .bind(options.limit)
        .fetch_all(&store.pool)
        .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(MeasurementRecord {
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            point_id: row.try_get("point_id")?,
            ts_ms: row.try_get("ts_ms")?,
            value: row.try_get("value")?,
            quality: row.try_get("quality")?,
        });
    }
    Ok(items)
}

async fn query_measurements_aggregated(
    store: &PgMeasurementStore,
    ctx: &TenantContext,
    project_id: &str,
    point_id: &str,
    options: MeasurementsQueryOptions,
    aggregation: crate::traits::MeasurementAggregation,
) -> Result<Vec<MeasurementRecord>, StorageError> {
    let bucket_ms = aggregation.bucket_ms;
    if bucket_ms <= 0 {
        return Ok(Vec::new());
    }
    let (cursor_op, order_by) = match options.order {
        TimeOrder::Asc => (">", "asc"),
        TimeOrder::Desc => ("<", "desc"),
    };

    let agg_expr = match aggregation.func {
        MeasurementAggFn::Avg => "avg(value::double precision)::text",
        MeasurementAggFn::Min => "min(value::double precision)::text",
        MeasurementAggFn::Max => "max(value::double precision)::text",
        MeasurementAggFn::Sum => "sum(value::double precision)::text",
        MeasurementAggFn::Count => "count(*)::text",
    };

    let sql = format!(
        "with filtered as ( \
            select tenant_id, project_id, point_id, ts, \
              to_timestamp(floor(extract(epoch from ts) * 1000 / $7) * $7 / 1000.0) as bucket_ts, \
              value \
            from measurement \
            where tenant_id = $1 \
            and project_id = $2 \
            and point_id = $3 \
            and ($4 is null or ts >= to_timestamp($4 / 1000.0)) \
            and ($5 is null or ts <= to_timestamp($5 / 1000.0)) \
         ) \
         select tenant_id, project_id, point_id, \
           (extract(epoch from bucket_ts) * 1000)::bigint as ts_ms, \
           {agg_expr} as value, \
           null::text as quality \
         from filtered \
         where ($6 is null or bucket_ts {cursor_op} to_timestamp($6 / 1000.0)) \
         group by tenant_id, project_id, point_id, bucket_ts \
         order by bucket_ts {order_by} \
         limit $8"
    );

    let rows = sqlx::query(&sql)
        .bind(&ctx.tenant_id)
        .bind(project_id)
        .bind(point_id)
        .bind(options.from_ms)
        .bind(options.to_ms)
        .bind(options.cursor_ts_ms)
        .bind(bucket_ms)
        .bind(options.limit)
        .fetch_all(&store.pool)
        .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(MeasurementRecord {
            tenant_id: row.try_get("tenant_id")?,
            project_id: row.try_get("project_id")?,
            point_id: row.try_get("point_id")?,
            ts_ms: row.try_get("ts_ms")?,
            value: row.try_get("value")?,
            quality: None,
        });
    }
    Ok(items)
}
