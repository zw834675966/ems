use api_contract::{LoginResponse, RefreshTokenRequest, RefreshTokenResponse};
use serde_json::Value;

#[test]
fn login_response_is_camel_case() {
    let response = LoginResponse {
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        expires: 1_700_000_000_000,
        username: "admin".to_string(),
        nickname: "admin".to_string(),
        avatar: "".to_string(),
        roles: vec!["admin".to_string()],
        permissions: vec![],
    };
    let value = serde_json::to_value(response).expect("serialize");
    assert!(value.get("accessToken").is_some());
    assert!(value.get("refreshToken").is_some());
    assert!(value.get("expires").is_some());
    assert!(value.get("access_token").is_none());
    assert!(value.get("refresh_token").is_none());
}

#[test]
fn refresh_token_request_accepts_camel_case() {
    let payload = r#"{"refreshToken":"token-1"}"#;
    let req: RefreshTokenRequest = serde_json::from_str(payload).expect("parse");
    assert_eq!(req.refresh_token, "token-1");
}

#[test]
fn refresh_token_request_accepts_snake_case() {
    let payload = r#"{"refresh_token":"token-2"}"#;
    let req: RefreshTokenRequest = serde_json::from_str(payload).expect("parse");
    assert_eq!(req.refresh_token, "token-2");
}

#[test]
fn refresh_token_response_is_camel_case() {
    let response = RefreshTokenResponse {
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        expires: 1_700_000_000_000,
    };
    let value = serde_json::to_value(response).expect("serialize");
    assert!(value.get("accessToken").is_some());
    assert!(value.get("refreshToken").is_some());
    assert!(value.get("expires").is_some());
    assert!(value.get("access_token").is_none());
    assert!(value.get("refresh_token").is_none());
}

#[test]
fn expires_is_number() {
    let response = LoginResponse {
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        expires: 1_700_000_000_000,
        username: "admin".to_string(),
        nickname: "admin".to_string(),
        avatar: "".to_string(),
        roles: vec![],
        permissions: vec![],
    };
    let value = serde_json::to_value(response).expect("serialize");
    let expires = value.get("expires").expect("expires");
    assert!(matches!(expires, Value::Number(_)));
}
