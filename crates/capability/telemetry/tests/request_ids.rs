use ems_telemetry::new_request_ids;

#[test]
fn request_ids_non_empty() {
    let ids = new_request_ids();
    assert!(!ids.request_id.is_empty());
    assert!(!ids.trace_id.is_empty());
}
