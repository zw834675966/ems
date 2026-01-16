use async_trait::async_trait;
use domain::RawEvent;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// 采集错误。
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("handler error: {0}")]
    Handler(String),
    #[error("source error: {0}")]
    Source(String),
}

/// RawEvent 处理器。
#[async_trait]
pub trait RawEventHandler: Send + Sync {
    async fn handle(&self, event: RawEvent) -> Result<(), IngestError>;
}

/// 采集源抽象。
#[async_trait]
pub trait Source: Send + Sync {
    async fn run(&self, handler: Arc<dyn RawEventHandler>) -> Result<(), IngestError>;
}

/// 占位源（用于接线与测试）。
#[derive(Debug, Default)]
pub struct NoopSource;

#[async_trait]
impl Source for NoopSource {
    async fn run(&self, _handler: Arc<dyn RawEventHandler>) -> Result<(), IngestError> {
        Ok(())
    }
}

/// MQTT 采集源配置。
#[derive(Debug, Clone)]
pub struct MqttSourceConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub topic_prefix: String,
    pub has_source_id: bool,
}

/// MQTT 采集源（占位实现）。
#[derive(Debug, Clone)]
pub struct MqttSource {
    config: MqttSourceConfig,
}

impl MqttSource {
    pub fn new(config: MqttSourceConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &MqttSourceConfig {
        &self.config
    }
}

#[async_trait]
impl Source for MqttSource {
    async fn run(&self, _handler: Arc<dyn RawEventHandler>) -> Result<(), IngestError> {
        let client_id = format!("ems-ingest-{}", now_epoch_ms());
        let mut options =
            rumqttc::MqttOptions::new(client_id, self.config.host.clone(), self.config.port);
        options.set_keep_alive(Duration::from_secs(30));
        if let (Some(username), Some(password)) =
            (self.config.username.as_ref(), self.config.password.as_ref())
        {
            options.set_credentials(username, password);
        }

        let (client, mut eventloop) = rumqttc::AsyncClient::new(options, 10);
        let topic = format!("{}/#", self.config.topic_prefix.trim_end_matches('/'));
        client
            .subscribe(topic, rumqttc::QoS::AtMostOnce)
            .await
            .map_err(|err| IngestError::Source(err.to_string()))?;

        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let (tenant_id, project_id, source_id, address) =
                        match extract_scope(&self.config.topic_prefix, &publish.topic, self.config.has_source_id) {
                            Some(scope) => scope,
                            None => {
                                warn!("mqtt topic skipped: {}", publish.topic);
                                continue;
                            }
                        };
                    let event = RawEvent {
                        tenant_id,
                        project_id,
                        source_id,
                        address,
                        payload: publish.payload.to_vec(),
                        received_at_ms: now_epoch_ms(),
                    };
                    if let Err(err) = _handler.handle(event).await {
                        warn!("raw event handler failed: {}", err);
                    }
                }
                Ok(_) => {}
                Err(err) => return Err(IngestError::Source(err.to_string())),
            }
        }
    }
}

fn extract_scope(prefix: &str, topic: &str, has_source_id: bool) -> Option<(String, String, String, String)> {
    let prefix = prefix.trim_matches('/');
    let topic = topic.trim_matches('/');
    let rest = if prefix.is_empty() {
        topic
    } else {
        topic.strip_prefix(prefix)?
    };
    let rest = rest.trim_start_matches('/');
    let mut parts = rest.split('/');
    let tenant_id = parts.next()?;
    let project_id = parts.next()?;
    let (source_id, address) = if has_source_id {
        let source_id = parts.next()?;
        let address = parts.collect::<Vec<_>>().join("/");
        if address.is_empty() {
            return None;
        }
        (source_id.to_string(), address)
    } else {
        let address = parts.collect::<Vec<_>>().join("/");
        ("".to_string(), address)
    };
    if address.is_empty() {
        return None;
    }
    Some((tenant_id.to_string(), project_id.to_string(), source_id, address))
}

fn now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}
