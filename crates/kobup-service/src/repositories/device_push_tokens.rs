use uuid::Uuid;

use crate::DbPool;

pub async fn list_all_tokens(pool: &DbPool) -> super::Result<Vec<String>> {
    let rows = sqlx::query!("SELECT token FROM device_push_tokens WHERE provider = 'fcm'",)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.token).collect())
}

pub async fn upsert(
    pool: &DbPool,
    device_id: &str,
    provider: &str,
    token: &str,
) -> super::Result<String> {
    let id = Uuid::now_v7().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query!(
        "INSERT INTO device_push_tokens (id, device_id, provider, token, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(device_id, provider) DO UPDATE SET token = excluded.token, updated_at = excluded.updated_at",
        id,
        device_id,
        provider,
        token,
        now,
    )
    .execute(pool)
    .await?;
    Ok(id)
}
