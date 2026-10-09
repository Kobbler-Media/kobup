use uuid::Uuid;

use crate::DbPool;

pub async fn create(
    pool: &DbPool,
    device_id: &str,
    package_name: &str,
    from_version_code: Option<i64>,
    to_version_code: Option<i64>,
    status: &str,
    message: Option<&str>,
) -> super::Result<String> {
    let id = Uuid::now_v7().to_string();
    sqlx::query!(
        "INSERT INTO install_events (id, device_id, package_name, from_version_code, to_version_code, status, message) VALUES (?, ?, ?, ?, ?, ?, ?)",
        id,
        device_id,
        package_name,
        from_version_code,
        to_version_code,
        status,
        message,
    )
    .execute(pool)
    .await?;
    Ok(id)
}
