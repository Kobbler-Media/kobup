use uuid::Uuid;

use super::hash_token;
use crate::DbPool;

pub async fn create(
    pool: &DbPool,
    user_id: &str,
    token: &str,
    scope: &str,
    expires_at: Option<&str>,
) -> super::Result<String> {
    let id = Uuid::now_v7().to_string();
    let token_hash = hash_token(token);
    sqlx::query!(
        "INSERT INTO auth_tokens (id, user_id, token_hash, scope, expires_at) VALUES (?, ?, ?, ?, ?)",
        id,
        user_id,
        token_hash,
        scope,
        expires_at,
    )
    .execute(pool)
    .await?;
    Ok(id)
}

/// Validate a token with the given scope. Returns the user_id if valid.
pub async fn validate_token(
    pool: &DbPool,
    token: &str,
    scope: &str,
) -> super::Result<Option<String>> {
    if token.is_empty() {
        return Ok(None);
    }
    let token_hash = hash_token(token);
    let row = sqlx::query_scalar!(
        "SELECT user_id FROM auth_tokens WHERE token_hash = ? AND scope = ? AND revoked = 0 AND (expires_at IS NULL OR expires_at > datetime('now'))",
        token_hash,
        scope,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn revoke_by_token(pool: &DbPool, token: &str) -> super::Result<bool> {
    let token_hash = hash_token(token);
    let result = sqlx::query!(
        "UPDATE auth_tokens SET revoked = 1 WHERE token_hash = ? AND revoked = 0",
        token_hash,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
