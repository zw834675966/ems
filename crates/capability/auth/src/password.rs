use crate::AuthError;
use argon2::{
    Argon2,
    PasswordHash,
    PasswordHasher,
    PasswordVerifier,
    password_hash::SaltString,
};
use rand_core::OsRng;
use subtle::ConstantTimeEq;

pub struct PasswordCheck {
    pub verified: bool,
    pub upgrade_hash: Option<String>,
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| AuthError::Internal(err.to_string()))?;
    Ok(hash.to_string())
}

pub fn verify_password_and_maybe_upgrade(
    stored_password_hash: &str,
    password: &str,
) -> Result<PasswordCheck, AuthError> {
    if stored_password_hash.starts_with("$argon2") {
        let parsed = PasswordHash::new(stored_password_hash)
            .map_err(|err| AuthError::Internal(err.to_string()))?;
        let argon2 = Argon2::default();
        let verified = argon2
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        return Ok(PasswordCheck {
            verified,
            upgrade_hash: None,
        });
    }

    let verified: bool = stored_password_hash.as_bytes().ct_eq(password.as_bytes()).into();
    if !verified {
        return Ok(PasswordCheck {
            verified: false,
            upgrade_hash: None,
        });
    }

    let new_hash = hash_password(password)?;
    Ok(PasswordCheck {
        verified: true,
        upgrade_hash: Some(new_hash),
    })
}
