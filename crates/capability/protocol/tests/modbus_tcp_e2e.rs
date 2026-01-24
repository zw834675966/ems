use async_trait::async_trait;
use ems_protocol::{
    ModbusTcpConfig, ModbusTcpSource, ProtocolError, ProtocolEvent, ProtocolEventHandler,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_modbus::server::tcp::{accept_tcp_connection, Server};
use tokio_modbus::server::Service;
use tokio_modbus::{ExceptionCode, Request, Response, SlaveRequest};

#[derive(Clone)]
struct TestModbusService {
    holding: Arc<Vec<u16>>,
    inputs: Arc<Vec<u16>>,
    coils: Arc<Vec<bool>>,
    discretes: Arc<Vec<bool>>,
}

impl TestModbusService {
    fn new() -> Self {
        Self {
            holding: Arc::new(vec![123, 456, 0x0E, 0xD6]),
            inputs: Arc::new(vec![111, 222]),
            coils: Arc::new(vec![true, false, true]),
            discretes: Arc::new(vec![false, true, false]),
        }
    }
}

impl Service for TestModbusService {
    type Request = SlaveRequest<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Exception>> + Send>,
    >;

    fn call(&self, req: Self::Request) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            let SlaveRequest { request, .. } = req;
            match request {
                Request::ReadHoldingRegisters(addr, qty) => {
                    let start = addr as usize;
                    let end = start.saturating_add(qty as usize);
                    if end > this.holding.len() {
                        return Err(ExceptionCode::IllegalDataAddress);
                    }
                    Ok(Response::ReadHoldingRegisters(
                        this.holding[start..end].to_vec(),
                    ))
                }
                Request::ReadInputRegisters(addr, qty) => {
                    let start = addr as usize;
                    let end = start.saturating_add(qty as usize);
                    if end > this.inputs.len() {
                        return Err(ExceptionCode::IllegalDataAddress);
                    }
                    Ok(Response::ReadInputRegisters(
                        this.inputs[start..end].to_vec(),
                    ))
                }
                Request::ReadCoils(addr, qty) => {
                    let start = addr as usize;
                    let end = start.saturating_add(qty as usize);
                    if end > this.coils.len() {
                        return Err(ExceptionCode::IllegalDataAddress);
                    }
                    Ok(Response::ReadCoils(this.coils[start..end].to_vec()))
                }
                Request::ReadDiscreteInputs(addr, qty) => {
                    let start = addr as usize;
                    let end = start.saturating_add(qty as usize);
                    if end > this.discretes.len() {
                        return Err(ExceptionCode::IllegalDataAddress);
                    }
                    Ok(Response::ReadDiscreteInputs(
                        this.discretes[start..end].to_vec(),
                    ))
                }
                _ => Err(ExceptionCode::IllegalFunction),
            }
        })
    }
}

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn modbus_tcp_reads_registers_and_emits_events() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let server = Server::new(listener);

    let server_task = tokio::spawn(async move {
        let on_connected = |stream, socket_addr| async move {
            accept_tcp_connection(stream, socket_addr, |_| Ok(Some(TestModbusService::new())))
        };
        server
            .serve(&on_connected, |_err: std::io::Error| {})
            .await
            .expect("serve");
    });

    let mut source = ModbusTcpSource::new(ModbusTcpConfig {
        host: "127.0.0.1".to_string(),
        port,
        poll_interval_ms: 50,
        connect_timeout_ms: 1000,
        request_timeout_ms: 500,
        max_retries: 0,
        retry_interval_ms: 0,
        reconnect_backoff_ms: 1000,
    });

    // 两个相邻 holding register 读取，验证批读与解析：
    // addr=0 => 123
    // addr=1 => 456
    source
        .add_task_from_config(
            "tenant-1",
            "project-1",
            "gw-1",
            "dev-1",
            "src-1",
            r#"{"unitId":1}"#,
            r#"{"functionCode":3,"registerAddress":0,"registerCount":1,"dataType":"uint16","endian":"big_endian"}"#,
            None,
            None,
        )
        .expect("task1");
    source
        .add_task_from_config(
            "tenant-1",
            "project-1",
            "gw-1",
            "dev-1",
            "src-2",
            r#"{"unitId":1}"#,
            r#"{"functionCode":3,"registerAddress":1,"registerCount":1,"dataType":"uint16","endian":"big_endian"}"#,
            None,
            None,
        )
        .expect("task2");

    // 读取 coils，bool -> 0/1
    source
        .add_task_from_config(
            "tenant-1",
            "project-1",
            "gw-1",
            "dev-1",
            "src-3",
            r#"{"unitId":1}"#,
            r#"{"functionCode":1,"registerAddress":0,"registerCount":1,"dataType":"bool","endian":"big_endian"}"#,
            None,
            None,
        )
        .expect("task3");

    let (tx, mut rx) = mpsc::channel::<ProtocolEvent>(16);
    let handler: Arc<dyn ProtocolEventHandler> = Arc::new(CollectHandler { tx });

    let run_task = tokio::spawn(async move { source.run(handler).await });

    // 收到至少 3 个 event
    let mut seen = std::collections::HashMap::<String, f64>::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while seen.len() < 3 && std::time::Instant::now() < deadline {
        if let Ok(Some(ev)) =
            tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await
        {
            seen.insert(ev.source_id.clone(), ev.value);
        }
    }

    assert_eq!(seen.get("src-1").copied(), Some(123.0));
    assert_eq!(seen.get("src-2").copied(), Some(456.0));
    assert_eq!(seen.get("src-3").copied(), Some(1.0));

    run_task.abort();
    server_task.abort();
}
