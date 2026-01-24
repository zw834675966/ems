use async_trait::async_trait;
use ems_ingest::{MqttSource, MqttSourceConfig, RawEventHandler, Source};
use std::sync::Arc;
use tokio::sync::mpsc;

struct CollectHandler {
    tx: mpsc::Sender<domain::RawEvent>,
}

#[async_trait]
impl RawEventHandler for CollectHandler {
    async fn handle(&self, event: domain::RawEvent) -> Result<(), ems_ingest::IngestError> {
        let _ = self.tx.send(event).await;
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mqtt_source_receives_publish_from_embedded_broker() {
    // 选一个空闲端口
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    // 启动内置 broker（rumqttd）
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().expect("addr");
    let mut cfg = rumqttd::Config::default();
    cfg.router.max_connections = 32;
    cfg.router.max_segment_size = 1024 * 1024;
    cfg.router.max_segment_count = 10;
    cfg.router.max_outgoing_packet_count = 1024;
    cfg.v4 = Some(std::collections::HashMap::from([(
        "test".to_string(),
        rumqttd::ServerSettings {
            name: "test".to_string(),
            listen: addr,
            tls: None,
            next_connection_delay_ms: 0,
            connections: rumqttd::ConnectionSettings {
                connection_timeout_ms: 1000,
                max_payload_size: 256 * 1024,
                max_inflight_count: 100,
                auth: None,
                external_auth: None,
                dynamic_filters: false,
            },
        },
    )]));

    std::thread::spawn(move || {
        let mut broker = rumqttd::Broker::new(cfg);
        let _ = broker.start();
    });

    // 等待 broker 监听起来
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 启动 MQTT source
    let source = MqttSource::new(MqttSourceConfig {
        host: "127.0.0.1".to_string(),
        port,
        username: None,
        password: None,
        topic_prefix: "ems/data".to_string(),
        has_source_id: true,
        use_shared_subscription: false,
        shared_group: "ems-ingest".to_string(),
        use_tls: false,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
    });

    let (tx, mut rx) = mpsc::channel::<domain::RawEvent>(8);
    let handler: Arc<dyn RawEventHandler> = Arc::new(CollectHandler { tx });
    let source_task = tokio::spawn(async move { source.run(handler).await });

    // 等待 source 完成连接与订阅
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // 发布消息
    let mut options = rumqttc::MqttOptions::new("publisher", "127.0.0.1", port);
    options.set_keep_alive(std::time::Duration::from_secs(5));
    let (client, mut eventloop) = rumqttc::AsyncClient::new(options, 10);

    let topic = "ems/data/tenant-1/project-1/source-1/device/1/telemetry";
    client
        .publish(topic, rumqttc::QoS::AtMostOnce, false, b"{\"value\":1}")
        .await
        .expect("publish");

    // 驱动 publisher eventloop 一下，确保发送
    for _ in 0..5 {
        let _ = tokio::time::timeout(std::time::Duration::from_millis(200), eventloop.poll()).await;
    }

    let ev = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout")
        .expect("event");

    assert_eq!(ev.tenant_id, "tenant-1");
    assert_eq!(ev.project_id, "project-1");
    assert_eq!(ev.source_id, "source-1");
    assert_eq!(ev.address, "device/1/telemetry");

    source_task.abort();
}
