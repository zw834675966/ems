use async_trait::async_trait;
use ems_protocol::{
    Endian, ProtocolError, ProtocolEvent, ProtocolEventHandler, TcpPointMapping, TcpServerConfig,
    TcpServerSource, TcpValueType,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

struct CollectHandler {
    tx: mpsc::Sender<ProtocolEvent>,
}

#[async_trait]
impl ProtocolEventHandler for CollectHandler {
    async fn handle(&self, event: ProtocolEvent) -> Result<(), ProtocolError> {
        let _ = self.tx.send(event).await;
        Ok(())
    }
}

fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

fn build_frame_one_device_one_tlv(dev_id: u16, dev_type: u8, tag: u8, value: &[u8]) -> Vec<u8> {
    let mut dev_payload = Vec::with_capacity(2 + value.len());
    dev_payload.push(tag);
    dev_payload.push(u8::try_from(value.len()).expect("value too long"));
    dev_payload.extend_from_slice(value);

    let mut payload = Vec::new();
    payload.push(0x01); // VER
    payload.extend_from_slice(&[0x00, 0x00]); // LEN placeholder
    payload.push(0x01); // DEV_COUNT
    payload.extend_from_slice(&dev_id.to_be_bytes());
    payload.push(dev_type);
    payload.extend_from_slice(&(dev_payload.len() as u16).to_be_bytes());
    payload.extend_from_slice(&dev_payload);

    // LEN is from VER to CRC before (include VER+LEN+DEV_COUNT+DEV_BLOCK)
    let frame_len = payload.len() as u16;
    payload[1..3].copy_from_slice(&frame_len.to_be_bytes());

    let crc = crc16_modbus(&payload);

    let mut frame = vec![0xAA, 0x55];
    frame.extend_from_slice(&payload);
    frame.extend_from_slice(&crc.to_be_bytes());
    frame
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tcp_server_receives_frame_and_emits_event() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let source = TcpServerSource::new(TcpServerConfig {
        listen_port: port,
        max_connections: 10,
        connection_timeout_secs: 300,
    });

    source
        .register_point_mapping(
            1,
            1,
            TcpPointMapping {
                tenant_id: "tenant-1".to_string(),
                project_id: "project-1".to_string(),
                gateway_id: "gw-1".to_string(),
                device_id: "dev-1".to_string(),
                source_id: "src-1".to_string(),
                value_type: TcpValueType::UInt16,
                endian: Endian::Big,
                scale: Some(0.1),
                offset: None,
            },
        )
        .await;

    let (tx, mut rx) = mpsc::channel::<ProtocolEvent>(8);
    let handler: Arc<dyn ProtocolEventHandler> = Arc::new(CollectHandler { tx });

    let run_task = tokio::spawn(async move { source.run(handler).await });

    // 等待 server 启动绑定
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");

    // TAG=1, UINT16(0x0ED6=3798) => after scale(0.1) => 379.8
    let frame = build_frame_one_device_one_tlv(1, 1, 1, &[0x0E, 0xD6]);
    stream.write_all(&frame).await.expect("write");

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("event");

    assert_eq!(event.tenant_id, "tenant-1");
    assert_eq!(event.project_id, "project-1");
    assert_eq!(event.gateway_id, "gw-1");
    assert_eq!(event.device_id, "dev-1");
    assert_eq!(event.source_id, "src-1");
    assert!((event.value - 379.8).abs() < 1e-6);

    run_task.abort();
}
