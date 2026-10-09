use serde::Serialize;

use crate::DbPool;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

pub async fn list(pool: &DbPool) -> super::Result<Vec<User>> {
    let rows = sqlx::query_as!(
        User,
        "SELECT id, email, role, created_at FROM users ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create(
    pool: &DbPool,
    id: &str,
    email: &str,
    password_hash: &str,
    role: &str,
) -> super::Result<User> {
    sqlx::query!(
        "INSERT INTO users (id, email, password_hash, role) VALUES (?, ?, ?, ?)",
        id,
        email,
        password_hash,
        role,
    )
    .execute(pool)
    .await?;

    // Return the created user (fetch to get the DB-generated created_at)
    let row = sqlx::query_as!(
        User,
        "SELECT id, email, role, created_at FROM users WHERE id = ?",
        id
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn delete(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Returns (user_id, password_hash, role) for login validation.
pub async fn get_by_email_for_login(
    pool: &DbPool,
    email: &str,
) -> super::Result<Option<(String, String, String)>> {
    let row = sqlx::query!(
        "SELECT id, password_hash, role FROM users WHERE email = ?",
        email,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| (r.id, r.password_hash, r.role)))
}

/// Returns (id, email, role).
pub async fn get_by_id(pool: &DbPool, id: &str) -> super::Result<Option<(String, String, String)>> {
    let row = sqlx::query!("SELECT id, email, role FROM users WHERE id = ?", id,)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| (r.id, r.email, r.role)))
}

pub async fn get_password_hash(pool: &DbPool, user_id: &str) -> super::Result<Option<String>> {
    let row = sqlx::query_scalar!("SELECT password_hash FROM users WHERE id = ?", user_id,)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn update_email(pool: &DbPool, user_id: &str, new_email: &str) -> super::Result<bool> {
    let result = sqlx::query!(
        "UPDATE users SET email = ? WHERE id = ?",
        new_email,
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_password(pool: &DbPool, user_id: &str, new_hash: &str) -> super::Result<bool> {
    let result = sqlx::query!(
        "UPDATE users SET password_hash = ? WHERE id = ?",
        new_hash,
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
