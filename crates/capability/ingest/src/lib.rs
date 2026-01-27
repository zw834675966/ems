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
    /// 是否启用共享订阅（支持水平扩容）
    pub use_shared_subscription: bool,
    /// 共享订阅组名（默认 ems-ingest）
    pub shared_group: String,
    /// 是否启用 TLS 加密连接
    pub use_tls: bool,
    /// CA 证书路径（用于验证服务器证书）
    pub ca_cert_path: Option<String>,
    /// 客户端证书路径（用于 mTLS 双向认证）
    pub client_cert_path: Option<String>,
    /// 客户端私钥路径（用于 mTLS 双向认证）
    pub client_key_path: Option<String>,
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
        // 商用场景下，MQTT 网络抖动/重连是常态。
        // 这里使用“永不退出”的重连循环：断线后指数退避重连，避免采集线程永久停止。
        let mut backoff_ms: u64 = 500;
        let max_backoff_ms: u64 = 30_000;

        loop {
            match self.run_once(_handler.clone()).await {
                Ok(()) => {
                    // 正常情况下 run_once 不会返回 Ok（内部是 poll loop）。
                    // 为兼容未来实现，返回 Ok 时直接退出。
                    return Ok(());
                }
                Err(err) => {
                    let jitter_ms = now_epoch_ms().rem_euclid(250) as u64;
                    warn!(
                        error = %err,
                        backoff_ms = backoff_ms,
                        jitter_ms = jitter_ms,
                        "mqtt ingest loop stopped, reconnecting"
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms.saturating_add(jitter_ms)))
                        .await;
                    backoff_ms = (backoff_ms.saturating_mul(2)).min(max_backoff_ms);
                }
            }
        }
    }
}

impl MqttSource {
    async fn run_once(&self, handler: Arc<dyn RawEventHandler>) -> Result<(), IngestError> {
        let client_id = format!("ems-ingest-{}", now_epoch_ms());
        let mut options =
            rumqttc::MqttOptions::new(client_id, self.config.host.clone(), self.config.port);
        options.set_keep_alive(Duration::from_secs(30));
        if let (Some(username), Some(password)) =
            (self.config.username.as_ref(), self.config.password.as_ref())
        {
            options.set_credentials(username, password);
        }

        // 配置 TLS 传输层（如果启用）
        if self.config.use_tls {
            let tls_config = build_tls_config(
                self.config.ca_cert_path.as_deref(),
                self.config.client_cert_path.as_deref(),
                self.config.client_key_path.as_deref(),
            )
            .map_err(|err| IngestError::Source(format!("TLS config error: {}", err)))?;
            options.set_transport(rumqttc::Transport::tls_with_config(tls_config));
        }

        let (client, mut eventloop) = rumqttc::AsyncClient::new(options, 10);
        // 构建订阅 Topic：如果启用共享订阅，使用 $share/<group>/<topic> 格式
        let base_topic = format!("{}/#", self.config.topic_prefix.trim_end_matches('/'));
        let topic = if self.config.use_shared_subscription {
            format!("$share/{}/{}", self.config.shared_group, base_topic)
        } else {
            base_topic
        };
        client
            .subscribe(topic, rumqttc::QoS::AtMostOnce)
            .await
            .map_err(|err| IngestError::Source(err.to_string()))?;

        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let (tenant_id, project_id, source_id, address) = match extract_scope(
                        &self.config.topic_prefix,
                        &publish.topic,
                        self.config.has_source_id,
                    ) {
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
                    if let Err(err) = handler.handle(event).await {
                        warn!("raw event handler failed: {}", err);
                    }
                }
                Ok(_) => {}
                Err(err) => return Err(IngestError::Source(err.to_string())),
            }
        }
    }
}

fn extract_scope(
    prefix: &str,
    topic: &str,
    has_source_id: bool,
) -> Option<(String, String, String, String)> {
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
    Some((
        tenant_id.to_string(),
        project_id.to_string(),
        source_id,
        address,
    ))
}

fn now_epoch_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}

/// 构建 TLS 配置
///
/// 根据提供的证书路径构建 rumqttc 的 TLS 配置：
/// - ca_cert_path: CA 证书路径（用于验证服务器证书）
/// - client_cert_path + client_key_path: 客户端证书和私钥（用于 mTLS）
fn build_tls_config(
    ca_cert_path: Option<&str>,
    client_cert_path: Option<&str>,
    client_key_path: Option<&str>,
) -> Result<rumqttc::TlsConfiguration, String> {
    use std::io::BufReader;

    // 读取 CA 证书（如果提供）
    let ca_certs = if let Some(ca_path) = ca_cert_path {
        let ca_file = std::fs::File::open(ca_path)
            .map_err(|e| format!("failed to open CA cert '{}': {}", ca_path, e))?;
        let mut reader = BufReader::new(ca_file);
        rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("failed to parse CA cert: {}", e))?
    } else {
        Vec::new()
    };

    // 读取客户端证书和私钥（如果提供 mTLS）
    let client_auth = match (client_cert_path, client_key_path) {
        (Some(cert_path), Some(key_path)) => {
            let cert_file = std::fs::File::open(cert_path)
                .map_err(|e| format!("failed to open client cert '{}': {}", cert_path, e))?;
            let mut cert_reader = BufReader::new(cert_file);
            let client_certs = rustls_pemfile::certs(&mut cert_reader)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("failed to parse client cert: {}", e))?;

            let key_file = std::fs::File::open(key_path)
                .map_err(|e| format!("failed to open client key '{}': {}", key_path, e))?;
            let mut key_reader = BufReader::new(key_file);
            let client_key = rustls_pemfile::private_key(&mut key_reader)
                .map_err(|e| format!("failed to parse client key: {}", e))?
                .ok_or_else(|| "no private key found in file".to_string())?;

            Some((client_certs, client_key))
        }
        _ => None,
    };

    Ok(rumqttc::TlsConfiguration::Rustls(std::sync::Arc::new(
        build_rustls_client_config(ca_certs, client_auth)?,
    )))
}

/// 构建 rustls ClientConfig
fn build_rustls_client_config(
    ca_certs: Vec<rustls_pki_types::CertificateDer<'static>>,
    client_auth: Option<(
        Vec<rustls_pki_types::CertificateDer<'static>>,
        rustls_pki_types::PrivateKeyDer<'static>,
    )>,
) -> Result<rustls::ClientConfig, String> {
    let mut root_store = rustls::RootCertStore::empty();

    // 添加自定义 CA 证书
    for cert in ca_certs {
        root_store
            .add(cert)
            .map_err(|e| format!("failed to add CA cert: {}", e))?;
    }

    // 如果没有自定义 CA，使用系统根证书
    if root_store.is_empty() {
        let native_certs = rustls_native_certs::load_native_certs()
            .map_err(|e| format!("could not load platform certs: {}", e))?;
        for cert in native_certs {
            root_store.add(cert).ok();
        }
    }

    let builder = rustls::ClientConfig::builder().with_root_certificates(root_store);

    // 配置客户端认证（mTLS）
    let config = if let Some((certs, key)) = client_auth {
        builder
            .with_client_auth_cert(certs, key)
            .map_err(|e| format!("failed to configure client auth: {}", e))?
    } else {
        builder.with_no_client_auth()
    };

    Ok(config)
}
