use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash};

pub fn hash_password(password: &str) -> Result<String, password_hash::Error> {
    Ok(Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string())
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_HASH: &str =
        "$argon2id$v=19$m=256,t=2,p=1$c29tZXNhbHQ$nf65EOgLrQMR/uIPnA4rEsF5h7TKyQwu9U1bMCHGi/4";

    #[test]
    fn verifies_hashes_stored_before_argon2_upgrade() {
        assert!(verify_password(LEGACY_HASH, "password"));
        assert!(!verify_password(LEGACY_HASH, "wrong"));
    }

    #[test]
    fn rejects_malformed_hash() {
        assert!(!verify_password("not-a-phc-string", "password"));
    }

    #[test]
    fn round_trips_with_unique_salts() {
        let a = hash_password("hunter42").unwrap();
        let b = hash_password("hunter42").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("$argon2id$v=19$"));
        assert!(verify_password(&a, "hunter42"));
        assert!(!verify_password(&a, "hunter43"));
    }
}
