//! Modbus snapshot (read + write) handler
//!
//! POST /projects/:project_id/modbus/snapshot

use crate::AppState;
use crate::middleware::{require_permission, require_project_scope};
use crate::utils::response::{bad_request_error, not_found_error};
use api_contract::{
    ApiError, ApiResponse, ModbusRawValue, ModbusReadItem, ModbusReadResult, ModbusSnapshotRequest,
    ModbusSnapshotResponse, ModbusSnapshotTarget, ModbusWriteItem, ModbusWriteResult, error_codes,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::permissions;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio_modbus::prelude::*;
use tracing::{debug, info, warn};

#[derive(serde::Deserialize)]
pub struct ProjectPath {
    project_id: String,
}

fn api_error(code: &str, message: impl Into<String>) -> ApiError {
    ApiError {
        code: code.to_string(),
        message: message.into(),
    }
}

fn response_error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (status, Json(ApiResponse::<()>::error(code, message.into()))).into_response()
}

fn response_error_with_data<T: serde::Serialize>(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    data: T,
) -> Response {
    (
        status,
        Json(ApiResponse::<T> {
            success: false,
            data: Some(data),
            error: Some(ApiError {
                code: code.to_string(),
                message: message.into(),
            }),
        }),
    )
        .into_response()
}

static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Semaphore>>>> = OnceLock::new();

async fn semaphore_for(key: &str) -> Arc<Semaphore> {
    let locks = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = locks.lock().await;
    guard
        .entry(key.to_string())
        .or_insert_with(|| Arc::new(Semaphore::new(1)))
        .clone()
}

fn validate_read_item(item: &ModbusReadItem) -> Result<(), ApiError> {
    let max = match item.function_code {
        1 | 2 => 2000u16,
        3 | 4 => 125u16,
        _ => {
            return Err(api_error(
                error_codes::MODBUS_INVALID_FUNCTION_CODE,
                format!("unsupported function code: {}", item.function_code),
            ));
        }
    };
    if item.quantity == 0 || item.quantity > max {
        return Err(api_error(
            error_codes::MODBUS_INVALID_QUANTITY,
            format!(
                "FC{:02} quantity must be within 1..={}",
                item.function_code, max
            ),
        ));
    }
    item.address
        .checked_add(item.quantity.saturating_sub(1))
        .ok_or_else(|| {
            api_error(
                error_codes::MODBUS_INVALID_ADDRESS_RANGE,
                "address range overflow",
            )
        })?;
    Ok(())
}

fn validate_write_item(item: &ModbusWriteItem) -> Result<(), ApiError> {
    match item.function_code {
        5 => {
            if item.value != 0 && item.value != 1 {
                return Err(api_error(
                    error_codes::MODBUS_INVALID_VALUE,
                    "FC05 value must be 0 or 1",
                ));
            }
        }
        6 => {
            // u16 already bounds 0..=65535
        }
        _ => {
            return Err(api_error(
                error_codes::MODBUS_INVALID_FUNCTION_CODE,
                format!("unsupported function code: {}", item.function_code),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    start: u16,
    count: u16,
}

fn merge_contiguous_with_max(items: &[ModbusReadItem], max_len: u16) -> Vec<Segment> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|i| i.address);

    let mut segments: Vec<Segment> = Vec::new();
    for item in sorted {
        // Split a single large item into chunks (e.g. quantity=2000 bits) so each request
        // never exceeds `max_len`.
        let mut remaining = item.quantity;
        let mut cursor = item.address;

        while remaining > 0 {
            let chunk = remaining.min(max_len);
            let chunk_end = cursor.saturating_add(chunk.saturating_sub(1));

            if let Some(last) = segments.last_mut() {
                let last_end = last.start.saturating_add(last.count.saturating_sub(1));
                let contiguous = cursor <= last_end.saturating_add(1);
                if contiguous {
                    let new_end = last_end.max(chunk_end);
                    let new_len = new_end.saturating_sub(last.start).saturating_add(1);
                    if new_len <= max_len {
                        last.count = new_len;
                        cursor = cursor.saturating_add(chunk);
                        remaining = remaining.saturating_sub(chunk);
                        continue;
                    }
                }
            }

            segments.push(Segment {
                start: cursor,
                count: chunk,
            });
            cursor = cursor.saturating_add(chunk);
            remaining = remaining.saturating_sub(chunk);
        }
    }
    segments
}

#[derive(Debug, Clone)]
struct ModbusOpError {
    status: StatusCode,
    api_error: ApiError,
}

fn classify_modbus_error(err: &tokio_modbus::Error, phase: &str) -> ModbusOpError {
    match err {
        tokio_modbus::Error::Transport(io) => ModbusOpError {
            status: StatusCode::BAD_GATEWAY,
            api_error: api_error(
                error_codes::MODBUS_EXCEPTION,
                format!("{phase} transport error: {io}"),
            ),
        },
        tokio_modbus::Error::Protocol(protocol) => match protocol {
            tokio_modbus::ProtocolError::FunctionCodeMismatch { request, result } => match result {
                Ok(resp) => ModbusOpError {
                    status: StatusCode::BAD_GATEWAY,
                    api_error: api_error(
                        error_codes::MODBUS_EXCEPTION,
                        format!("{phase} function code mismatch (request={request:?}): {resp:?}"),
                    ),
                },
                Err(exc) => ModbusOpError {
                    status: StatusCode::BAD_GATEWAY,
                    api_error: api_error(
                        error_codes::MODBUS_EXCEPTION,
                        format!(
                            "{phase} exception: function={:?} code={:?} ({})",
                            exc.function, exc.exception, exc
                        ),
                    ),
                },
            },
            tokio_modbus::ProtocolError::HeaderMismatch { message, result } => match result {
                Ok(resp) => ModbusOpError {
                    status: StatusCode::BAD_GATEWAY,
                    api_error: api_error(
                        error_codes::MODBUS_EXCEPTION,
                        format!("{phase} header mismatch: {message}; {resp:?}"),
                    ),
                },
                Err(exc) => ModbusOpError {
                    status: StatusCode::BAD_GATEWAY,
                    api_error: api_error(
                        error_codes::MODBUS_EXCEPTION,
                        format!(
                            "{phase} exception (header mismatch): function={:?} code={:?} ({})",
                            exc.function, exc.exception, exc
                        ),
                    ),
                },
            },
        },
    }
}

async fn connect_with_timeout(
    host: &str,
    port: u16,
    connect_timeout_ms: u64,
) -> Result<tokio_modbus::client::Context, ModbusOpError> {
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|_| ModbusOpError {
            status: StatusCode::BAD_REQUEST,
            api_error: api_error(error_codes::MODBUS_CONFIG_MISSING, "invalid host/port"),
        })?;

    let res = tokio::time::timeout(
        Duration::from_millis(connect_timeout_ms),
        tcp::connect(addr),
    )
    .await;
    match res {
        Err(_) => Err(ModbusOpError {
            status: StatusCode::GATEWAY_TIMEOUT,
            api_error: api_error(error_codes::MODBUS_CONNECT_TIMEOUT, "connect timeout"),
        }),
        Ok(Err(err)) => Err(ModbusOpError {
            status: StatusCode::BAD_GATEWAY,
            api_error: api_error(
                error_codes::MODBUS_EXCEPTION,
                format!("connect transport error: {err}"),
            ),
        }),
        Ok(Ok(ctx)) => Ok(ctx),
    }
}

fn exception_error(exc: &tokio_modbus::ExceptionCode, phase: &str) -> ModbusOpError {
    ModbusOpError {
        status: StatusCode::BAD_GATEWAY,
        api_error: api_error(
            error_codes::MODBUS_EXCEPTION,
            format!("{phase} modbus exception: {exc:?} ({exc})"),
        ),
    }
}

async fn read_bits_with_retries(
    ctx: &mut tokio_modbus::client::Context,
    function_code: u8,
    start: u16,
    count: u16,
    request_timeout_ms: u64,
    max_retries: u32,
    retry_interval_ms: u64,
) -> Result<Vec<bool>, ModbusOpError> {
    let mut attempt = 0u32;
    loop {
        let timeout_res = tokio::time::timeout(Duration::from_millis(request_timeout_ms), async {
            match function_code {
                1 => ctx.read_coils(start, count).await,
                2 => ctx.read_discrete_inputs(start, count).await,
                _ => unreachable!("validated"),
            }
        })
        .await;

        let res = match timeout_res {
            Err(_) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(ModbusOpError {
                        status: StatusCode::GATEWAY_TIMEOUT,
                        api_error: api_error(
                            error_codes::MODBUS_REQUEST_TIMEOUT,
                            "modbus request timeout",
                        ),
                    });
                }
                None
            }
            Ok(res) => Some(res),
        };

        if let Some(res) = res {
            let res = match res {
                Err(err) => {
                    attempt += 1;
                    if attempt > max_retries {
                        return Err(classify_modbus_error(&err, "read"));
                    }
                    None
                }
                Ok(res) => Some(res),
            };

            if let Some(res) = res {
                match res {
                    Ok(values) => return Ok(values),
                    Err(exc) => return Err(exception_error(&exc, "read")),
                }
            }
        }

        if retry_interval_ms > 0 {
            tokio::time::sleep(Duration::from_millis(retry_interval_ms)).await;
        }
    }
}

async fn read_registers_with_retries(
    ctx: &mut tokio_modbus::client::Context,
    function_code: u8,
    start: u16,
    count: u16,
    request_timeout_ms: u64,
    max_retries: u32,
    retry_interval_ms: u64,
) -> Result<Vec<u16>, ModbusOpError> {
    let mut attempt = 0u32;
    loop {
        let timeout_res = tokio::time::timeout(Duration::from_millis(request_timeout_ms), async {
            match function_code {
                3 => ctx.read_holding_registers(start, count).await,
                4 => ctx.read_input_registers(start, count).await,
                _ => unreachable!("validated"),
            }
        })
        .await;

        let res = match timeout_res {
            Err(_) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(ModbusOpError {
                        status: StatusCode::GATEWAY_TIMEOUT,
                        api_error: api_error(
                            error_codes::MODBUS_REQUEST_TIMEOUT,
                            "modbus request timeout",
                        ),
                    });
                }
                None
            }
            Ok(res) => Some(res),
        };

        if let Some(res) = res {
            let res = match res {
                Err(err) => {
                    attempt += 1;
                    if attempt > max_retries {
                        return Err(classify_modbus_error(&err, "read"));
                    }
                    None
                }
                Ok(res) => Some(res),
            };

            if let Some(res) = res {
                match res {
                    Ok(values) => return Ok(values),
                    Err(exc) => return Err(exception_error(&exc, "read")),
                }
            }
        }

        if retry_interval_ms > 0 {
            tokio::time::sleep(Duration::from_millis(retry_interval_ms)).await;
        }
    }
}

async fn write_single_coil_with_retries(
    ctx: &mut tokio_modbus::client::Context,
    address: u16,
    value: bool,
    request_timeout_ms: u64,
    max_retries: u32,
    retry_interval_ms: u64,
) -> Result<(), ModbusOpError> {
    let mut attempt = 0u32;
    loop {
        let timeout_res = tokio::time::timeout(Duration::from_millis(request_timeout_ms), async {
            ctx.write_single_coil(address, value).await
        })
        .await;

        let res = match timeout_res {
            Err(_) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(ModbusOpError {
                        status: StatusCode::GATEWAY_TIMEOUT,
                        api_error: api_error(
                            error_codes::MODBUS_REQUEST_TIMEOUT,
                            "modbus write timeout",
                        ),
                    });
                }
                None
            }
            Ok(res) => Some(res),
        };

        if let Some(res) = res {
            let res = match res {
                Err(err) => {
                    attempt += 1;
                    if attempt > max_retries {
                        return Err(classify_modbus_error(&err, "write"));
                    }
                    None
                }
                Ok(res) => Some(res),
            };

            if let Some(res) = res {
                match res {
                    Ok(()) => return Ok(()),
                    Err(exc) => return Err(exception_error(&exc, "write")),
                }
            }
        }

        if retry_interval_ms > 0 {
            tokio::time::sleep(Duration::from_millis(retry_interval_ms)).await;
        }
    }
}

async fn write_single_register_with_retries(
    ctx: &mut tokio_modbus::client::Context,
    address: u16,
    value: u16,
    request_timeout_ms: u64,
    max_retries: u32,
    retry_interval_ms: u64,
) -> Result<(), ModbusOpError> {
    let mut attempt = 0u32;
    loop {
        let timeout_res = tokio::time::timeout(Duration::from_millis(request_timeout_ms), async {
            ctx.write_single_register(address, value).await
        })
        .await;

        let res = match timeout_res {
            Err(_) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(ModbusOpError {
                        status: StatusCode::GATEWAY_TIMEOUT,
                        api_error: api_error(
                            error_codes::MODBUS_REQUEST_TIMEOUT,
                            "modbus write timeout",
                        ),
                    });
                }
                None
            }
            Ok(res) => Some(res),
        };

        if let Some(res) = res {
            let res = match res {
                Err(err) => {
                    attempt += 1;
                    if attempt > max_retries {
                        return Err(classify_modbus_error(&err, "write"));
                    }
                    None
                }
                Ok(res) => Some(res),
            };

            if let Some(res) = res {
                match res {
                    Ok(()) => return Ok(()),
                    Err(exc) => return Err(exception_error(&exc, "write")),
                }
            }
        }

        if retry_interval_ms > 0 {
            tokio::time::sleep(Duration::from_millis(retry_interval_ms)).await;
        }
    }
}

fn update_overall_error(overall: &mut Option<ModbusOpError>, err: ModbusOpError) {
    // Prefer timeouts over transport/protocol as "most severe" for our use case.
    // We keep the first timeout, otherwise the first error.
    if overall.is_none() {
        *overall = Some(err);
        return;
    }
    let Some(existing) = overall.as_ref() else {
        return;
    };
    let existing_is_timeout = existing.api_error.code == error_codes::MODBUS_CONNECT_TIMEOUT
        || existing.api_error.code == error_codes::MODBUS_REQUEST_TIMEOUT;
    let new_is_timeout = err.api_error.code == error_codes::MODBUS_CONNECT_TIMEOUT
        || err.api_error.code == error_codes::MODBUS_REQUEST_TIMEOUT;
    if !existing_is_timeout && new_is_timeout {
        *overall = Some(err);
    }
}

/// Modbus snapshot (read + write).
pub async fn modbus_snapshot(
    State(state): State<AppState>,
    Path(path): Path<ProjectPath>,
    headers: HeaderMap,
    Json(req): Json<ModbusSnapshotRequest>,
) -> Response {
    let ctx = match require_project_scope(&state, &headers, &path.project_id).await {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    let wants_reads = !req.reads.is_empty();
    let wants_writes = !req.writes.is_empty();
    if !wants_reads && !wants_writes {
        return bad_request_error("reads/writes required");
    }

    // Reads require realtime read permission; writes require command issue permission.
    if wants_reads && let Err(response) = require_permission(&ctx, permissions::DATA_REALTIME_READ)
    {
        return response;
    }
    if wants_writes
        && let Err(response) = require_permission(&ctx, permissions::CONTROL_COMMAND_ISSUE)
    {
        return response;
    }

    for item in &req.reads {
        if let Err(err) = validate_read_item(item) {
            return response_error(StatusCode::BAD_REQUEST, &err.code, err.message);
        }
    }
    for item in &req.writes {
        if let Err(err) = validate_write_item(item) {
            return response_error(StatusCode::BAD_REQUEST, &err.code, err.message);
        }
    }

    // Lookup gateway (must be modbus_tcp) and device (optional).
    let gateway = match state
        .gateway_store
        .find_gateway(&ctx, &path.project_id, &req.gateway_id)
        .await
    {
        Ok(Some(gw)) => gw,
        Ok(None) => return not_found_error(),
        Err(err) => return crate::utils::response::storage_error(err),
    };
    if gateway.protocol_type != "modbus_tcp" {
        return response_error(
            StatusCode::BAD_REQUEST,
            error_codes::MODBUS_CONFIG_MISSING,
            "gateway is not modbus_tcp",
        );
    }

    let cfg: ems_protocol::ModbusTcpConfig = match gateway.protocol_config.as_deref() {
        Some(s) => match serde_json::from_str(s) {
            Ok(v) => v,
            Err(e) => {
                return response_error(
                    StatusCode::BAD_REQUEST,
                    error_codes::MODBUS_CONFIG_MISSING,
                    format!("invalid gateway protocolConfig: {e}"),
                );
            }
        },
        None => {
            return response_error(
                StatusCode::BAD_REQUEST,
                error_codes::MODBUS_CONFIG_MISSING,
                "missing gateway protocolConfig",
            );
        }
    };

    if cfg.host.trim().is_empty() {
        return response_error(
            StatusCode::BAD_REQUEST,
            error_codes::MODBUS_CONFIG_MISSING,
            "gateway protocolConfig.host required",
        );
    }

    let mut unit_id: Option<u8> = req.unit_id;
    let device_id: Option<String> = req.device_id.clone();
    if let Some(did) = req.device_id.as_deref() {
        let device = match state
            .device_store
            .find_device(&ctx, &path.project_id, did)
            .await
        {
            Ok(Some(dev)) => dev,
            Ok(None) => {
                return response_error(
                    StatusCode::BAD_REQUEST,
                    error_codes::MODBUS_CONFIG_MISSING,
                    "device not found",
                );
            }
            Err(err) => return crate::utils::response::storage_error(err),
        };
        if device.gateway_id != gateway.gateway_id {
            return response_error(
                StatusCode::BAD_REQUEST,
                error_codes::MODBUS_CONFIG_MISSING,
                "device.gatewayId does not match gatewayId",
            );
        }
        if unit_id.is_none() {
            let addr_json = device.address_config.as_deref().unwrap_or("{}");
            let addr: ems_protocol::ModbusDeviceAddress = match serde_json::from_str(addr_json) {
                Ok(v) => v,
                Err(_) => {
                    return response_error(
                        StatusCode::BAD_REQUEST,
                        error_codes::MODBUS_CONFIG_MISSING,
                        "device addressConfig.unitId required",
                    );
                }
            };
            unit_id = Some(addr.unit_id);
        }
    }

    let unit_id = match unit_id {
        Some(id) if (1..=247).contains(&id) => id,
        _ => {
            return response_error(
                StatusCode::BAD_REQUEST,
                error_codes::MODBUS_CONFIG_MISSING,
                "unitId required (1..=247)",
            );
        }
    };

    let connect_timeout_ms = req.connect_timeout_ms.unwrap_or(cfg.connect_timeout_ms);
    let request_timeout_ms = req.request_timeout_ms.unwrap_or(cfg.request_timeout_ms);
    let max_retries = req.max_retries.unwrap_or(cfg.max_retries);
    let retry_interval_ms = req.retry_interval_ms.unwrap_or(cfg.retry_interval_ms);
    let lock_timeout_ms = req.lock_timeout_ms.unwrap_or(500);

    // Suggested optimization: lock by gatewayId + unitId for stability (host can change).
    let sem_key = format!("{}:{}", gateway.gateway_id, unit_id);
    let sem = semaphore_for(&sem_key).await;
    let lock_start = Instant::now();
    let permit =
        match tokio::time::timeout(Duration::from_millis(lock_timeout_ms), sem.acquire()).await {
            Ok(Ok(permit)) => permit,
            _ => {
                return response_error(
                    StatusCode::CONFLICT,
                    error_codes::MODBUS_DEVICE_BUSY,
                    "device busy",
                );
            }
        };
    let _permit = permit;
    let waited_ms = lock_start.elapsed().as_millis() as i64;

    let start = Instant::now();
    info!(
        target: "ems.modbus_snapshot",
        tenant_id = %ctx.tenant_id,
        project_id = %path.project_id,
        gateway_id = %req.gateway_id,
        unit_id = unit_id,
        waited_ms = waited_ms,
        reads = req.reads.len(),
        writes = req.writes.len(),
        "modbus_snapshot_start"
    );

    let mut overall_error: Option<ModbusOpError> = None;
    let mut modbus = match connect_with_timeout(&cfg.host, cfg.port, connect_timeout_ms).await {
        Ok(ctx) => ctx,
        Err(err) => {
            update_overall_error(&mut overall_error, err.clone());
            let resp = ModbusSnapshotResponse {
                captured_at_ms: chrono::Utc::now().timestamp_millis(),
                latency_ms: start.elapsed().as_millis() as i64,
                target: ModbusSnapshotTarget {
                    gateway_id: gateway.gateway_id,
                    device_id,
                    unit_id,
                    host: cfg.host,
                    port: cfg.port,
                },
                results: Vec::new(),
                write_results: req
                    .writes
                    .iter()
                    .map(|w| ModbusWriteResult {
                        function_code: w.function_code,
                        address: w.address,
                        value: w.value,
                        ok: false,
                        error: Some(err.api_error.clone()),
                    })
                    .collect(),
            };
            return response_error_with_data(
                err.status,
                &err.api_error.code,
                err.api_error.message,
                resp,
            );
        }
    };
    modbus.set_slave(Slave(unit_id));

    // Writes: execute all, collect results; never early-return.
    let mut write_results: Vec<ModbusWriteResult> = Vec::new();
    for w in &req.writes {
        let mut out = ModbusWriteResult {
            function_code: w.function_code,
            address: w.address,
            value: w.value,
            ok: false,
            error: None,
        };

        let res = match w.function_code {
            5 => {
                write_single_coil_with_retries(
                    &mut modbus,
                    w.address,
                    w.value == 1,
                    request_timeout_ms,
                    max_retries,
                    retry_interval_ms,
                )
                .await
            }
            6 => {
                write_single_register_with_retries(
                    &mut modbus,
                    w.address,
                    w.value,
                    request_timeout_ms,
                    max_retries,
                    retry_interval_ms,
                )
                .await
            }
            _ => Err(ModbusOpError {
                status: StatusCode::BAD_REQUEST,
                api_error: api_error(
                    error_codes::MODBUS_INVALID_FUNCTION_CODE,
                    format!("unsupported function code: {}", w.function_code),
                ),
            }),
        };

        match res {
            Ok(()) => {
                out.ok = true;
            }
            Err(err) => {
                warn!(
                    target: "ems.modbus_snapshot",
                    gateway_id = %gateway.gateway_id,
                    unit_id = unit_id,
                    function_code = w.function_code,
                    address = w.address,
                    error_code = %err.api_error.code,
                    error_message = %err.api_error.message,
                    "modbus_write_failed"
                );
                out.ok = false;
                out.error = Some(err.api_error.clone());
                update_overall_error(&mut overall_error, err);
            }
        }
        write_results.push(out);
    }

    // Reads: group by function code, merge contiguous segments with max limits, partial results allowed.
    let mut results: Vec<ModbusReadResult> = Vec::new();
    let mut reads_by_fc: HashMap<u8, Vec<ModbusReadItem>> = HashMap::new();
    for r in &req.reads {
        reads_by_fc
            .entry(r.function_code)
            .or_default()
            .push(r.clone());
    }

    // For each fc, store per-address value maps and per-segment errors.
    for (fc, items) in reads_by_fc {
        // Merge reads for contiguous areas, but never exceed device limits.
        // Spec max is 2000 bits for FC01/02 and 125 registers for FC03/04; we choose a safer
        // operational cap of 512 bits to better match typical field devices.
        let max_len = if fc == 1 || fc == 2 { 512u16 } else { 125u16 };
        let segments = merge_contiguous_with_max(&items, max_len);
        debug!(
            target: "ems.modbus_snapshot",
            gateway_id = %gateway.gateway_id,
            unit_id = unit_id,
            function_code = fc,
            segments = segments.len(),
            "merged_segments"
        );

        let mut bit_map: HashMap<u16, bool> = HashMap::new();
        let mut reg_map: HashMap<u16, u16> = HashMap::new();
        let mut seg_errors: Vec<(u16, u16, ApiError)> = Vec::new(); // (start,count,error)

        for seg in &segments {
            match fc {
                1 | 2 => match read_bits_with_retries(
                    &mut modbus,
                    fc,
                    seg.start,
                    seg.count,
                    request_timeout_ms,
                    max_retries,
                    retry_interval_ms,
                )
                .await
                {
                    Ok(values) => {
                        for (idx, v) in values.into_iter().enumerate() {
                            let addr = seg.start.saturating_add(idx as u16);
                            bit_map.insert(addr, v);
                        }
                    }
                    Err(err) => {
                        warn!(
                            target: "ems.modbus_snapshot",
                            gateway_id = %gateway.gateway_id,
                            unit_id = unit_id,
                            function_code = fc,
                            start = seg.start,
                            count = seg.count,
                            error_code = %err.api_error.code,
                            error_message = %err.api_error.message,
                            "modbus_read_segment_failed"
                        );
                        seg_errors.push((seg.start, seg.count, err.api_error.clone()));
                        update_overall_error(&mut overall_error, err);
                    }
                },
                3 | 4 => match read_registers_with_retries(
                    &mut modbus,
                    fc,
                    seg.start,
                    seg.count,
                    request_timeout_ms,
                    max_retries,
                    retry_interval_ms,
                )
                .await
                {
                    Ok(values) => {
                        for (idx, v) in values.into_iter().enumerate() {
                            let addr = seg.start.saturating_add(idx as u16);
                            reg_map.insert(addr, v);
                        }
                    }
                    Err(err) => {
                        warn!(
                            target: "ems.modbus_snapshot",
                            gateway_id = %gateway.gateway_id,
                            unit_id = unit_id,
                            function_code = fc,
                            start = seg.start,
                            count = seg.count,
                            error_code = %err.api_error.code,
                            error_message = %err.api_error.message,
                            "modbus_read_segment_failed"
                        );
                        seg_errors.push((seg.start, seg.count, err.api_error.clone()));
                        update_overall_error(&mut overall_error, err);
                    }
                },
                _ => {}
            }
        }

        let find_err_for_range = |start: u16, count: u16| -> Option<ApiError> {
            let end = start.saturating_add(count.saturating_sub(1));
            for (s, c, e) in &seg_errors {
                let seg_end = s.saturating_add(c.saturating_sub(1));
                let overlap = !(end < *s || start > seg_end);
                if overlap {
                    return Some(e.clone());
                }
            }
            None
        };

        for item in items {
            let mut raw = ModbusRawValue {
                bits: None,
                registers: None,
            };
            let mut item_err = find_err_for_range(item.address, item.quantity);

            match fc {
                1 | 2 => {
                    let mut out: Vec<bool> = Vec::with_capacity(item.quantity as usize);
                    for i in 0..item.quantity {
                        let addr = item.address.saturating_add(i);
                        if let Some(v) = bit_map.get(&addr).copied() {
                            out.push(v);
                        } else {
                            // if any missing, but no seg error recorded, treat as exception
                            if item_err.is_none() {
                                item_err = Some(api_error(
                                    error_codes::MODBUS_EXCEPTION,
                                    "missing bit value",
                                ));
                            }
                            out.push(false);
                        }
                    }
                    raw.bits = Some(out);
                }
                3 | 4 => {
                    let mut out: Vec<u16> = Vec::with_capacity(item.quantity as usize);
                    for i in 0..item.quantity {
                        let addr = item.address.saturating_add(i);
                        if let Some(v) = reg_map.get(&addr).copied() {
                            out.push(v);
                        } else {
                            if item_err.is_none() {
                                item_err = Some(api_error(
                                    error_codes::MODBUS_EXCEPTION,
                                    "missing register value",
                                ));
                            }
                            out.push(0);
                        }
                    }
                    raw.registers = Some(out);
                }
                _ => {}
            }

            results.push(ModbusReadResult {
                function_code: fc,
                address: item.address,
                quantity: item.quantity,
                raw,
                error: item_err,
            });
        }
    }

    let captured_at_ms = chrono::Utc::now().timestamp_millis();
    let latency_ms = start.elapsed().as_millis() as i64;
    info!(
        target: "ems.modbus_snapshot",
        tenant_id = %ctx.tenant_id,
        project_id = %path.project_id,
        gateway_id = %gateway.gateway_id,
        unit_id = unit_id,
        latency_ms = latency_ms,
        "modbus_snapshot_done"
    );

    let payload = ModbusSnapshotResponse {
        captured_at_ms,
        latency_ms,
        target: ModbusSnapshotTarget {
            gateway_id: gateway.gateway_id,
            device_id,
            unit_id,
            host: cfg.host,
            port: cfg.port,
        },
        results,
        write_results,
    };

    if let Some(err) = overall_error {
        return response_error_with_data(
            err.status,
            &err.api_error.code,
            err.api_error.message,
            payload,
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(payload))).into_response()
}
