use uuid::Uuid;

use super::hash_token;
use crate::DbPool;

pub struct ApiToken {
    pub id: String,
    pub project_id: Option<String>,
    pub name: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

pub async fn list(pool: &DbPool) -> super::Result<Vec<ApiToken>> {
    let rows = sqlx::query_as!(
        ApiToken,
        "SELECT id, project_id, name, created_at, revoked_at FROM api_tokens ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create(
    pool: &DbPool,
    project_id: Option<&str>,
    name: &str,
    token: &str,
) -> super::Result<String> {
    let id = Uuid::now_v7().to_string();
    let token_hash = hash_token(token);
    sqlx::query!(
        "INSERT INTO api_tokens (id, project_id, name, token_hash) VALUES (?, ?, ?, ?)",
        id,
        project_id,
        name,
        token_hash,
    )
    .execute(pool)
    .await?;
    Ok(id)
}

/// Validate an API token. Returns Some(project_id) if valid, None if invalid.
pub async fn validate(pool: &DbPool, token: &str) -> super::Result<Option<Option<String>>> {
    if token.is_empty() {
        return Ok(None);
    }
    let token_hash = hash_token(token);
    let row = sqlx::query!(
        "SELECT project_id FROM api_tokens WHERE token_hash = ? AND revoked_at IS NULL",
        token_hash,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.project_id))
}

pub async fn revoke(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!(
        "UPDATE api_tokens SET revoked_at = datetime('now') WHERE id = ? AND revoked_at IS NULL",
        id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
