use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DbPool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Release {
    pub id: String,
    pub project_id: String,
    pub channel_id: String,
    pub version_code: i64,
    pub version_name: String,
    pub release_notes: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub published_at: Option<String>,
    pub created_at: String,
}

pub struct CreateRelease {
    pub project_id: String,
    pub channel_id: String,
    pub version_code: i64,
    pub version_name: String,
    pub release_notes: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
}

pub async fn list_by_project(pool: &DbPool, project_id: &str) -> super::Result<Vec<Release>> {
    let rows = sqlx::query_as!(
        Release,
        "SELECT id, project_id, channel_id, version_code, version_name, release_notes, source_type, source_ref, published_at, created_at FROM releases WHERE project_id = ? ORDER BY version_code DESC",
        project_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_by_id(pool: &DbPool, id: &str) -> super::Result<Option<Release>> {
    let row = sqlx::query_as!(
        Release,
        "SELECT id, project_id, channel_id, version_code, version_name, release_notes, source_type, source_ref, published_at, created_at FROM releases WHERE id = ?",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn create(pool: &DbPool, input: CreateRelease) -> super::Result<Release> {
    let id = Uuid::now_v7().to_string();
    sqlx::query!(
        "INSERT INTO releases (id, project_id, channel_id, version_code, version_name, release_notes, source_type, source_ref) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        id,
        input.project_id,
        input.channel_id,
        input.version_code,
        input.version_name,
        input.release_notes,
        input.source_type,
        input.source_ref,
    )
    .execute(pool)
    .await?;

    Ok(Release {
        id,
        project_id: input.project_id,
        channel_id: input.channel_id,
        version_code: input.version_code,
        version_name: input.version_name,
        release_notes: input.release_notes,
        source_type: input.source_type,
        source_ref: input.source_ref,
        published_at: None,
        created_at: String::new(), // Will be set by SQLite default
    })
}

pub async fn publish(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!(
        "UPDATE releases SET published_at = datetime('now') WHERE id = ? AND published_at IS NULL",
        id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!("DELETE FROM releases WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_latest_published(
    pool: &DbPool,
    project_id: &str,
    channel_id: &str,
) -> super::Result<Option<Release>> {
    let row = sqlx::query_as!(
        Release,
        "SELECT id, project_id, channel_id, version_code, version_name, release_notes, source_type, source_ref, published_at, created_at FROM releases WHERE project_id = ? AND channel_id = ? AND published_at IS NOT NULL ORDER BY version_code DESC LIMIT 1",
        project_id,
        channel_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Returns releases to prune: those beyond the `keep` newest per project+channel.
pub async fn list_for_pruning(
    pool: &DbPool,
    project_id: &str,
    channel_id: &str,
    keep: i64,
) -> super::Result<Vec<Release>> {
    let rows = sqlx::query_as!(
        Release,
        "SELECT id, project_id, channel_id, version_code, version_name, release_notes, source_type, source_ref, published_at, created_at FROM releases WHERE project_id = ? AND channel_id = ? ORDER BY version_code DESC LIMIT -1 OFFSET ?",
        project_id,
        channel_id,
        keep,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn exists_version_code(
    pool: &DbPool,
    project_id: &str,
    channel_id: &str,
    version_code: i64,
) -> super::Result<bool> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) as count FROM releases WHERE project_id = ? AND channel_id = ? AND version_code = ?",
        project_id,
        channel_id,
        version_code,
    )
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
