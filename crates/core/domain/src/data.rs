/// 协议输入原始事件。
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub tenant_id: String,
    pub project_id: String,
    pub source_id: String,
    pub address: String,
    pub payload: Vec<u8>,
    pub received_at_ms: i64,
}

/// 点位值的数据类型。
#[derive(Debug, Clone)]
pub enum PointValueData {
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
}

/// 规范化后的点位值。
#[derive(Debug, Clone)]
pub struct PointValue {
    pub tenant_id: String,
    pub project_id: String,
    pub point_id: String,
    pub ts_ms: i64,
    pub value: PointValueData,
    pub quality: Option<String>,
}
