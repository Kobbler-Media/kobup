use sha2::{Digest, Sha256};

pub mod api_tokens;
pub mod artifacts;
pub mod auth_tokens;
pub mod channels;
pub mod device_push_tokens;
pub mod devices;
pub mod install_events;
pub mod projects;
pub mod releases;
pub mod users;

pub type Result<T> = std::result::Result<T, sqlx::Error>;

pub(crate) fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::hash_token;

    #[test]
    fn hash_token_matches_stored_production_hashes() {
        for (input, expected) in [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "kobup-token-119",
                "00db0ae52ab8a2f2661ad8298d6128d7a0d15bb7c958141f6f98ecb44fd60b11",
            ),
        ] {
            let actual = hash_token(input);
            assert_eq!(actual, expected);
            assert_eq!(actual.len(), 64);
            assert!(
                actual
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            );
        }
    }
}
