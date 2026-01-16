use async_trait::async_trait;
use domain::TenantContext;
use ems_telemetry::{
    record_command_dispatch_failure, record_command_dispatch_success, record_command_issue_latency_ms,
    record_command_issued, record_receipt_processed,
};
use ems_storage::{
    AuditLogRecord, AuditLogStore, CommandReceiptRecord, CommandReceiptStore, CommandRecord,
    CommandReceiptWriteResult, CommandStore,
};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// 命令下发请求。
#[derive(Debug, Clone)]
pub struct CommandRequest {
    pub project_id: String,
    pub target: String,
    pub payload: serde_json::Value,
    pub issued_at_ms: i64,
}

/// 命令下发数据。
#[derive(Debug, Clone)]
pub struct CommandDispatch {
    pub command_id: String,
    pub tenant_id: String,
    pub project_id: String,
    pub target: String,
    pub payload: String,
    pub issued_at_ms: i64,
}

/// 控制链路错误。
#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("dispatch error: {0}")]
    Dispatch(String),
    #[error("payload error: {0}")]
    Payload(String),
}

/// 命令下发器抽象。
#[async_trait]
pub trait CommandDispatcher: Send + Sync {
    async fn dispatch(&self, command: &CommandDispatch) -> Result<(), ControlError>;
}

/// 空下发器（用于占位）。
#[derive(Debug, Default)]
pub struct NoopDispatcher;

#[async_trait]
impl CommandDispatcher for NoopDispatcher {
    async fn dispatch(&self, _command: &CommandDispatch) -> Result<(), ControlError> {
        Ok(())
    }
}

/// MQTT Dispatcher 配置。
#[derive(Debug, Clone)]
pub struct MqttDispatcherConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub command_topic_prefix: String,
    /// 是否在命令 topic 中包含 target（用于设备侧按 target 订阅）。
    ///
    /// - off（默认）：`{prefix}/{tenant}/{project}/{command_id}`
    /// - on：`{prefix}/{tenant}/{project}/{target}/{command_id}`（target 可包含多段）
    pub include_target_in_topic: bool,
    pub qos: u8,
}

/// MQTT Dispatcher 实现（发布命令）。
#[derive(Clone)]
pub struct MqttDispatcher {
    client: AsyncClient,
    command_topic_prefix: String,
    include_target_in_topic: bool,
    qos: QoS,
}

impl MqttDispatcher {
    pub fn connect(
        config: MqttDispatcherConfig,
    ) -> Result<(Self, tokio::task::JoinHandle<()>), ControlError> {
        let client_id = format!("ems-control-dispatch-{}", uuid::Uuid::new_v4());
        let mut options = MqttOptions::new(client_id, config.host, config.port);
        options.set_keep_alive(Duration::from_secs(30));
        if let (Some(username), Some(password)) = (config.username, config.password) {
            options.set_credentials(username, password);
        }
        let (client, mut eventloop) = AsyncClient::new(options, 10);
        let handle = tokio::spawn(async move {
            loop {
                if let Err(err) = eventloop.poll().await {
                    warn!(target: "ems.control", "mqtt dispatch eventloop error: {}", err);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });
        Ok((
            Self {
                client,
                command_topic_prefix: config.command_topic_prefix,
                include_target_in_topic: config.include_target_in_topic,
                qos: qos_from_u8(config.qos),
            },
            handle,
        ))
    }

    fn topic_for(&self, tenant_id: &str, project_id: &str, target: &str, command_id: &str) -> String {
        let prefix = self.command_topic_prefix.trim_end_matches('/');
        if self.include_target_in_topic {
            let target = target.trim_matches('/');
            format!("{}/{}/{}/{}/{}", prefix, tenant_id, project_id, target, command_id)
        } else {
            format!("{}/{}/{}/{}", prefix, tenant_id, project_id, command_id)
        }
    }
}

#[async_trait]
impl CommandDispatcher for MqttDispatcher {
    async fn dispatch(&self, command: &CommandDispatch) -> Result<(), ControlError> {
        let topic = self.topic_for(
            &command.tenant_id,
            &command.project_id,
            &command.target,
            &command.command_id,
        );
        let payload = mqtt_command_payload(command)?;
        info!(
            target: "ems.control",
            tenant_id = %command.tenant_id,
            project_id = %command.project_id,
            command_id = %command.command_id,
            command_target = %command.target,
            topic = %topic,
            payload_size = payload.len(),
            "command_dispatch_publish"
        );
        self.client
            .publish(topic, self.qos, false, payload)
            .await
            .map_err(|err| ControlError::Dispatch(err.to_string()))?;
        Ok(())
    }
}

/// MQTT 回执监听配置。
#[derive(Debug, Clone)]
pub struct MqttReceiptListenerConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub receipt_topic_prefix: String,
    pub qos: u8,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptPayload {
    #[serde(alias = "result", alias = "state")]
    status: String,
    #[serde(alias = "msg", alias = "detail")]
    message: Option<String>,
    #[serde(
        alias = "tsMs",
        alias = "ts_ms",
        alias = "ts",
        alias = "timestamp",
        alias = "timeMs"
    )]
    ts_ms: Option<i64>,
}

pub fn spawn_receipt_listener(
    config: MqttReceiptListenerConfig,
    command_store: Arc<dyn CommandStore>,
    receipt_store: Arc<dyn CommandReceiptStore>,
    audit_store: Arc<dyn AuditLogStore>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let client_id = format!("ems-control-receipt-{}", uuid::Uuid::new_v4());
        let mut options = MqttOptions::new(client_id, config.host, config.port);
        options.set_keep_alive(Duration::from_secs(30));
        if let (Some(username), Some(password)) = (config.username, config.password) {
            options.set_credentials(username, password);
        }
        let (client, mut eventloop) = AsyncClient::new(options, 10);
        let topic = format!("{}/#", config.receipt_topic_prefix.trim_end_matches('/'));
        if let Err(err) = client.subscribe(topic, qos_from_u8(config.qos)).await {
            warn!(target: "ems.control", "mqtt receipt subscribe error: {}", err);
            return;
        }

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let Some((tenant_id, project_id, command_id)) =
                        extract_receipt_scope(&config.receipt_topic_prefix, &publish.topic)
                    else {
                        warn!(target: "ems.control", "receipt topic skipped: {}", publish.topic);
                        continue;
                    };
                    let payload = match parse_receipt_payload(&publish.payload) {
                        Ok(payload) => payload,
                        Err(err) => {
                            warn!(target: "ems.control", "receipt payload invalid: {}", err);
                            continue;
                        }
                    };
                    let ts_ms = payload.ts_ms.unwrap_or_else(now_epoch_ms);
                    let status = normalize_status(&payload.status);
                    let ctx = TenantContext::new(
                        tenant_id.clone(),
                        "system".to_string(),
                        Vec::new(),
                        Vec::new(),
                        Some(project_id.clone()),
                    );
                    let receipt = CommandReceiptRecord {
                        receipt_id: stable_receipt_id(
                            &tenant_id,
                            &project_id,
                            &command_id,
                            ts_ms,
                            &status,
                            payload.message.as_deref(),
                        ),
                        tenant_id: tenant_id.clone(),
                        project_id: project_id.clone(),
                        command_id: command_id.clone(),
                        ts_ms,
                        status: status.clone(),
                        message: payload.message.clone(),
                    };
                    let written: CommandReceiptWriteResult =
                        match receipt_store.create_receipt(&ctx, receipt).await {
                            Ok(result) => result,
                            Err(err) => {
                                warn!(target: "ems.control", "receipt write failed: {}", err);
                                continue;
                            }
                        };
                    if !written.inserted {
                        info!(
                            target: "ems.control",
                            tenant_id = %tenant_id,
                            project_id = %project_id,
                            command_id = %command_id,
                            receipt_id = %written.record.receipt_id,
                            "receipt_duplicate_ignored"
                        );
                        continue;
                    }
                    record_receipt_processed();
                    let _ = command_store
                        .update_command_status(&ctx, &project_id, &command_id, &status)
                        .await;
                    let audit = AuditLogRecord {
                        audit_id: stable_audit_id_for_receipt(&written.record.receipt_id),
                        tenant_id: tenant_id.clone(),
                        project_id: Some(project_id.clone()),
                        actor: "system".to_string(),
                        action: "CONTROL.COMMAND.RECEIPT".to_string(),
                        resource: format!("command:{}", command_id),
                        result: status.clone(),
                        detail: payload.message.clone(),
                        ts_ms,
                    };
                    let _ = audit_store.create_audit_log(&ctx, audit).await;
                    info!(
                        target: "ems.control",
                        tenant_id = %tenant_id,
                        project_id = %project_id,
                        command_id = %command_id,
                        status = %status,
                        message = ?payload.message,
                        ts_ms = ts_ms,
                        "receipt_processed"
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    warn!(target: "ems.control", "mqtt receipt eventloop error: {}", err);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    })
}

/// 命令服务（创建 + 下发 + 审计）。
#[derive(Clone)]
pub struct CommandService {
    command_store: Arc<dyn CommandStore>,
    audit_store: Arc<dyn AuditLogStore>,
    dispatcher: Arc<dyn CommandDispatcher>,
    config: CommandServiceConfig,
}

#[derive(Debug, Clone)]
pub struct CommandServiceConfig {
    pub dispatch_max_retries: u64,
    pub dispatch_backoff_ms: u64,
    /// 等待设备回执的超时（毫秒）。到期仍为 `accepted` 则自动流转为 `timeout`。
    pub receipt_timeout_ms: u64,
}

impl Default for CommandServiceConfig {
    fn default() -> Self {
        Self {
            dispatch_max_retries: 0,
            dispatch_backoff_ms: 0,
            receipt_timeout_ms: 0,
        }
    }
}

impl CommandService {
    pub fn new(
        command_store: Arc<dyn CommandStore>,
        audit_store: Arc<dyn AuditLogStore>,
        dispatcher: Arc<dyn CommandDispatcher>,
    ) -> Self {
        Self::new_with_config(command_store, audit_store, dispatcher, CommandServiceConfig::default())
    }

    pub fn new_with_config(
        command_store: Arc<dyn CommandStore>,
        audit_store: Arc<dyn AuditLogStore>,
        dispatcher: Arc<dyn CommandDispatcher>,
        config: CommandServiceConfig,
    ) -> Self {
        Self {
            command_store,
            audit_store,
            dispatcher,
            config,
        }
    }

    pub async fn issue_command(
        &self,
        ctx: &TenantContext,
        request: CommandRequest,
    ) -> Result<CommandRecord, ControlError> {
        record_command_issued();
        let started_at = Instant::now();
        let payload = serde_json::to_string(&request.payload)
            .map_err(|err| ControlError::Payload(err.to_string()))?;
        let command_id = uuid::Uuid::new_v4().to_string();
        info!(
            target: "ems.control",
            tenant_id = %ctx.tenant_id,
            project_id = %request.project_id,
            command_id = %command_id,
            actor = %ctx.user_id,
            command_target = %request.target,
            payload_size = payload.len(),
            issued_at_ms = request.issued_at_ms,
            "command_issue_requested"
        );
        let record = CommandRecord {
            command_id: command_id.clone(),
            tenant_id: ctx.tenant_id.clone(),
            project_id: request.project_id.clone(),
            target: request.target,
            payload: payload.clone(),
            status: "issued".to_string(),
            issued_by: ctx.user_id.clone(),
            issued_at_ms: request.issued_at_ms,
        };
        let record = self
            .command_store
            .create_command(ctx, record)
            .await
            .map_err(|err| ControlError::Storage(err.to_string()))?;
        info!(
            target: "ems.control",
            tenant_id = %record.tenant_id,
            project_id = %record.project_id,
            command_id = %record.command_id,
            status = %record.status,
            "command_created"
        );

        let dispatch = CommandDispatch {
            command_id: record.command_id.clone(),
            tenant_id: record.tenant_id.clone(),
            project_id: record.project_id.clone(),
            target: record.target.clone(),
            payload,
            issued_at_ms: record.issued_at_ms,
        };
        let (status, result, detail) = match dispatch_with_retry(
            self.dispatcher.clone(),
            &dispatch,
            self.config.dispatch_max_retries,
            self.config.dispatch_backoff_ms,
        )
        .await
        {
            Ok(()) => {
                record_command_dispatch_success();
                ("accepted", "success", None)
            }
            Err(err) => {
                record_command_dispatch_failure();
                ("failed", "failed", Some(err.to_string()))
            }
        };
        info!(
            target: "ems.control",
            tenant_id = %record.tenant_id,
            project_id = %record.project_id,
            command_id = %record.command_id,
            status = %status,
            result = %result,
            detail = ?detail,
            "command_dispatched"
        );
        let updated = self
            .command_store
            .update_command_status(ctx, &record.project_id, &record.command_id, status)
            .await
            .map_err(|err| ControlError::Storage(err.to_string()))?;
        let record = updated.unwrap_or_else(|| CommandRecord {
            status: status.to_string(),
            ..record
        });
        record_command_issue_latency_ms(started_at.elapsed().as_millis() as u64);

        if status == "accepted" && self.config.receipt_timeout_ms > 0 {
            spawn_command_timeout_task(
                self.command_store.clone(),
                self.audit_store.clone(),
                ctx.clone(),
                record.clone(),
                self.config.receipt_timeout_ms,
            );
        }

        let audit = AuditLogRecord {
            audit_id: uuid::Uuid::new_v4().to_string(),
            tenant_id: ctx.tenant_id.clone(),
            project_id: Some(record.project_id.clone()),
            actor: ctx.user_id.clone(),
            action: "CONTROL.COMMAND.ISSUE".to_string(),
            resource: format!("command:{}", record.command_id),
            result: result.to_string(),
            detail,
            ts_ms: record.issued_at_ms,
        };
        let _ = self.audit_store.create_audit_log(ctx, audit).await;
        Ok(record)
    }
}

fn spawn_command_timeout_task(
    command_store: Arc<dyn CommandStore>,
    audit_store: Arc<dyn AuditLogStore>,
    ctx: TenantContext,
    command: CommandRecord,
    timeout_ms: u64,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
        let transitioned = match command_store
            .transition_command_status(
                &ctx,
                &command.project_id,
                &command.command_id,
                "accepted",
                "timeout",
            )
            .await
        {
            Ok(changed) => changed,
            Err(err) => {
                warn!(
                    target: "ems.control",
                    tenant_id = %ctx.tenant_id,
                    project_id = %command.project_id,
                    command_id = %command.command_id,
                    error = %err,
                    "command_timeout_transition_failed"
                );
                return;
            }
        };

        if !transitioned {
            return;
        }

        let ts_ms = now_epoch_ms();
        let audit = AuditLogRecord {
            audit_id: uuid::Uuid::new_v4().to_string(),
            tenant_id: ctx.tenant_id.clone(),
            project_id: Some(command.project_id.clone()),
            actor: "system".to_string(),
            action: "CONTROL.COMMAND.TIMEOUT".to_string(),
            resource: format!("command:{}", command.command_id),
            result: "timeout".to_string(),
            detail: None,
            ts_ms,
        };
        let _ = audit_store.create_audit_log(&ctx, audit).await;
        info!(
            target: "ems.control",
            tenant_id = %ctx.tenant_id,
            project_id = %command.project_id,
            command_id = %command.command_id,
            timeout_ms = timeout_ms,
            "command_timed_out"
        );
    });
}

fn extract_receipt_scope(prefix: &str, topic: &str) -> Option<(String, String, String)> {
    let prefix = prefix.trim_matches('/');
    let topic = topic.trim_matches('/');
    let rest = if prefix.is_empty() {
        topic
    } else {
        topic.strip_prefix(prefix)?
    };
    let rest = rest.trim_start_matches('/');
    let parts: Vec<&str> = rest
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let tenant_id = parts[0];
    let project_id = parts[1];
    let command_id = parts[parts.len() - 1];
    Some((tenant_id.to_string(), project_id.to_string(), command_id.to_string()))
}

#[derive(Debug, Clone)]
struct ParsedReceiptPayload {
    status: String,
    message: Option<String>,
    ts_ms: Option<i64>,
}

fn parse_receipt_payload(payload: &[u8]) -> Result<ParsedReceiptPayload, String> {
    if payload.is_empty() {
        return Err("empty payload".to_string());
    }

    // Allow plain-text status (e.g. "success") for device compatibility.
    if let Ok(text) = std::str::from_utf8(payload) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("empty payload".to_string());
        }
        if !trimmed.starts_with('{') && !trimmed.starts_with('\"') {
            return Ok(ParsedReceiptPayload {
                status: trimmed.to_string(),
                message: None,
                ts_ms: None,
            });
        }
    }

    // JSON string status, e.g. "success"
    if let Ok(status) = serde_json::from_slice::<String>(payload) {
        let status = status.trim();
        if status.is_empty() {
            return Err("empty status".to_string());
        }
        return Ok(ParsedReceiptPayload {
            status: status.to_string(),
            message: None,
            ts_ms: None,
        });
    }

    let receipt: ReceiptPayload =
        serde_json::from_slice(payload).map_err(|err| err.to_string())?;
    if receipt.status.trim().is_empty() {
        return Err("missing status".to_string());
    }
    Ok(ParsedReceiptPayload {
        status: receipt.status,
        message: receipt.message,
        ts_ms: receipt.ts_ms,
    })
}

fn now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}

fn qos_from_u8(value: u8) -> QoS {
    match value {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtLeastOnce,
    }
}

fn normalize_status(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "accepted" => "accepted".to_string(),
        "success" => "success".to_string(),
        "failed" => "failed".to_string(),
        "timeout" => "timeout".to_string(),
        _ => "failed".to_string(),
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandMqttEnvelope<'a> {
    command_id: &'a str,
    target: &'a str,
    issued_at_ms: i64,
    payload: serde_json::Value,
}

fn mqtt_command_payload(command: &CommandDispatch) -> Result<Vec<u8>, ControlError> {
    let payload_value: serde_json::Value = serde_json::from_str(&command.payload)
        .unwrap_or_else(|_| serde_json::Value::String(command.payload.clone()));
    let envelope = CommandMqttEnvelope {
        command_id: &command.command_id,
        target: &command.target,
        issued_at_ms: command.issued_at_ms,
        payload: payload_value,
    };
    serde_json::to_vec(&envelope).map_err(|err| ControlError::Payload(err.to_string()))
}

fn stable_receipt_id(
    tenant_id: &str,
    project_id: &str,
    command_id: &str,
    ts_ms: i64,
    status: &str,
    message: Option<&str>,
) -> String {
    let name = format!(
        "receipt:{}:{}:{}:{}:{}:{}",
        tenant_id,
        project_id,
        command_id,
        ts_ms,
        status,
        message.unwrap_or("")
    );
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name.as_bytes()).to_string()
}

fn stable_audit_id_for_receipt(receipt_id: &str) -> String {
    let name = format!("audit:receipt:{}", receipt_id);
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_topic_scope_allows_extra_segments() {
        let prefix = "ems/receipts";
        let topic = "ems/receipts/tenant-1/project-1/demo-target/cmd-1";
        let scope = extract_receipt_scope(prefix, topic).expect("scope");
        assert_eq!(scope.0, "tenant-1");
        assert_eq!(scope.1, "project-1");
        assert_eq!(scope.2, "cmd-1");
    }

    #[test]
    fn receipt_payload_parses_json_object() {
        let payload = br#"{"status":"success","message":"ok","tsMs":1700000000000}"#;
        let parsed = parse_receipt_payload(payload).expect("parsed");
        assert_eq!(normalize_status(&parsed.status), "success");
        assert_eq!(parsed.message.as_deref(), Some("ok"));
        assert_eq!(parsed.ts_ms, Some(1_700_000_000_000));
    }

    #[test]
    fn receipt_payload_parses_plain_text_status() {
        let parsed = parse_receipt_payload(b"success").expect("parsed");
        assert_eq!(normalize_status(&parsed.status), "success");
        assert!(parsed.message.is_none());
        assert!(parsed.ts_ms.is_none());
    }
}

async fn dispatch_with_retry(
    dispatcher: Arc<dyn CommandDispatcher>,
    dispatch: &CommandDispatch,
    max_retries: u64,
    backoff_ms: u64,
) -> Result<(), ControlError> {
    let mut attempt = 0u64;
    loop {
        match dispatcher.dispatch(dispatch).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(err);
                }
                if backoff_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }
}
