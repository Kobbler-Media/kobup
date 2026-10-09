use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DbPool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Channel {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub is_default: bool,
    pub created_at: String,
}

pub async fn list_by_project(pool: &DbPool, project_id: &str) -> super::Result<Vec<Channel>> {
    let rows = sqlx::query_as!(
        Channel,
        r#"SELECT id, project_id, name, is_default as "is_default: bool", created_at FROM channels WHERE project_id = ? ORDER BY created_at ASC"#,
        project_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_by_id(pool: &DbPool, id: &str) -> super::Result<Option<Channel>> {
    let row = sqlx::query_as!(
        Channel,
        r#"SELECT id, project_id, name, is_default as "is_default: bool", created_at FROM channels WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn create(
    pool: &DbPool,
    project_id: &str,
    name: &str,
    is_default: bool,
) -> super::Result<Channel> {
    let id = Uuid::now_v7().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query!(
        "INSERT INTO channels (id, project_id, name, is_default, created_at) VALUES (?, ?, ?, ?, ?)",
        id,
        project_id,
        name,
        is_default,
        now,
    )
    .execute(pool)
    .await?;

    Ok(Channel {
        id,
        project_id: project_id.to_string(),
        name: name.to_string(),
        is_default,
        created_at: now,
    })
}

pub async fn get_by_name(
    pool: &DbPool,
    project_id: &str,
    name: &str,
) -> super::Result<Option<Channel>> {
    let row = sqlx::query_as!(
        Channel,
        r#"SELECT id, project_id, name, is_default as "is_default: bool", created_at FROM channels WHERE project_id = ? AND name = ?"#,
        project_id,
        name,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn delete(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!("DELETE FROM channels WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
