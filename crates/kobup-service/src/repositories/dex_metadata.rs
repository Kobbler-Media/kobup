use serde::{Deserialize, Serialize};

use crate::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DexMetadata {
    pub artifact_id: String,
    pub min_api: i64,
    pub sha256: String,
    pub size_bytes: i64,
    pub file_path: String,
}

pub async fn create(pool: &DbPool, dm: &DexMetadata) -> super::Result<()> {
    sqlx::query!(
        "INSERT INTO dex_metadata (artifact_id, min_api, sha256, size_bytes, file_path) VALUES (?, ?, ?, ?, ?)",
        dm.artifact_id,
        dm.min_api,
        dm.sha256,
        dm.size_bytes,
        dm.file_path,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get(
    pool: &DbPool,
    artifact_id: &str,
    min_api: i64,
) -> super::Result<Option<DexMetadata>> {
    let row = sqlx::query_as!(
        DexMetadata,
        "SELECT artifact_id, min_api, sha256, size_bytes, file_path FROM dex_metadata WHERE artifact_id = ? AND min_api = ?",
        artifact_id,
        min_api,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Each profile covers its own `min_api` up to the next one's, so a device takes the
/// highest band it has reached.
pub async fn get_for_sdk(
    pool: &DbPool,
    artifact_id: &str,
    sdk_int: i64,
) -> super::Result<Option<DexMetadata>> {
    let row = sqlx::query_as!(
        DexMetadata,
        "SELECT artifact_id, min_api, sha256, size_bytes, file_path FROM dex_metadata WHERE artifact_id = ? AND min_api <= ? ORDER BY min_api DESC LIMIT 1",
        artifact_id,
        sdk_int,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
