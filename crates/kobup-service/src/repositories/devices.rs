use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DbPool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Device {
    pub id: String,
    pub device_name: String,
    pub platform: String,
    pub sdk_int: Option<i64>,
    pub abis_json: String,
    pub is_tv: bool,
    pub kobup_version: Option<String>,
    pub app_version_code: Option<i64>,
    pub last_seen_at: String,
    pub created_at: String,
}

pub async fn list(pool: &DbPool) -> super::Result<Vec<Device>> {
    let rows = sqlx::query_as!(
        Device,
        r#"SELECT id, device_name, platform, sdk_int, abis_json, is_tv as "is_tv: bool", kobup_version, app_version_code, last_seen_at, created_at FROM devices ORDER BY last_seen_at DESC"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_by_id(pool: &DbPool, id: &str) -> super::Result<Option<Device>> {
    let row = sqlx::query_as!(
        Device,
        r#"SELECT id, device_name, platform, sdk_int, abis_json, is_tv as "is_tv: bool", kobup_version, app_version_code, last_seen_at, created_at FROM devices WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub struct CreateDeviceInput {
    pub device_name: String,
    pub sdk_int: Option<i64>,
    pub abis_json: String,
    pub is_tv: bool,
    pub kobup_version: Option<String>,
}

pub async fn create(pool: &DbPool, input: &CreateDeviceInput) -> super::Result<Device> {
    let id = Uuid::now_v7().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query!(
        "INSERT INTO devices (id, device_name, platform, sdk_int, abis_json, is_tv, kobup_version, last_seen_at, created_at) VALUES (?, ?, 'android', ?, ?, ?, ?, ?, ?)",
        id,
        input.device_name,
        input.sdk_int,
        input.abis_json,
        input.is_tv,
        input.kobup_version,
        now,
        now,
    )
    .execute(pool)
    .await?;

    Ok(Device {
        id,
        device_name: input.device_name.clone(),
        platform: "android".to_string(),
        sdk_int: input.sdk_int,
        abis_json: input.abis_json.clone(),
        is_tv: input.is_tv,
        kobup_version: input.kobup_version.clone(),
        app_version_code: None,
        last_seen_at: now.clone(),
        created_at: now,
    })
}

pub async fn update_heartbeat(
    pool: &DbPool,
    id: &str,
    kobup_version: Option<&str>,
    app_version_code: Option<i64>,
) -> super::Result<bool> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let result = sqlx::query!(
        "UPDATE devices SET last_seen_at = ?, kobup_version = COALESCE(?, kobup_version), app_version_code = COALESCE(?, app_version_code) WHERE id = ?",
        now,
        kobup_version,
        app_version_code,
        id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
