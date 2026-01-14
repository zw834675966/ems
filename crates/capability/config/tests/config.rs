use ems_config::AppConfig;

#[test]
fn load_config_from_env() {
    // Rust 2024 中 set_var 需要显式标注 unsafe（测试进程内可控）。
    unsafe {
        std::env::set_var("EMS_JWT_SECRET", "secret");
        std::env::set_var("EMS_JWT_ACCESS_TTL_SECONDS", "3600");
        std::env::set_var("EMS_JWT_REFRESH_TTL_SECONDS", "7200");
        std::env::set_var("EMS_HTTP_ADDR", "127.0.0.1:8081");
    }

    let config = AppConfig::from_env().expect("config");
    assert_eq!(config.http_addr, "127.0.0.1:8081");
    assert_eq!(config.jwt_access_ttl_seconds, 3600);
    assert_eq!(config.jwt_refresh_ttl_seconds, 7200);
}
