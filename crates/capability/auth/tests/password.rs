use ems_auth::{hash_password, verify_password_and_maybe_upgrade};

#[test]
fn argon2_hash_verifies() {
    let hash = hash_password("admin123").expect("hash");
    let check = verify_password_and_maybe_upgrade(&hash, "admin123").expect("check");
    assert!(check.verified);
    assert!(check.upgrade_hash.is_none());
}

#[test]
fn legacy_plaintext_upgrades() {
    let stored = "admin123";
    let check = verify_password_and_maybe_upgrade(stored, "admin123").expect("check");
    assert!(check.verified);
    assert!(check.upgrade_hash.as_deref().unwrap_or_default().starts_with("$argon2"));
}

#[test]
fn wrong_password_rejected() {
    let stored = "admin123";
    let check = verify_password_and_maybe_upgrade(stored, "bad").expect("check");
    assert!(!check.verified);
    assert!(check.upgrade_hash.is_none());
}

