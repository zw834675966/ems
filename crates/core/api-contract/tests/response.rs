use api_contract::ApiResponse;

#[test]
fn api_response_success() {
    let response = ApiResponse::success("ok");
    assert!(response.success);
    assert!(response.data.is_some());
    assert!(response.error.is_none());
}

#[test]
fn api_response_error() {
    let response = ApiResponse::<()>::error("AUTH.UNAUTHORIZED", "unauthorized");
    assert!(!response.success);
    assert!(response.data.is_none());
    assert!(response.error.is_some());
}
