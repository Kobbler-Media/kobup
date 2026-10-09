use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DbPool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub package_name: String,
    pub visibility: String,
    pub default_channel: String,
    pub source_type: String,
    pub source_config_json: Option<String>,
    pub allowed_signing_fingerprints_json: Option<String>,
    pub icon_path: Option<String>,
    pub max_releases_per_channel: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list(pool: &DbPool) -> super::Result<Vec<Project>> {
    let rows = sqlx::query_as!(
        Project,
        "SELECT id, slug, name, description, package_name, visibility, default_channel, source_type, source_config_json, allowed_signing_fingerprints_json, icon_path, max_releases_per_channel, created_at, updated_at FROM projects ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_by_id(pool: &DbPool, id: &str) -> super::Result<Option<Project>> {
    let row = sqlx::query_as!(
        Project,
        "SELECT id, slug, name, description, package_name, visibility, default_channel, source_type, source_config_json, allowed_signing_fingerprints_json, icon_path, max_releases_per_channel, created_at, updated_at FROM projects WHERE id = ?",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_by_slug(pool: &DbPool, slug: &str) -> super::Result<Option<Project>> {
    let row = sqlx::query_as!(
        Project,
        "SELECT id, slug, name, description, package_name, visibility, default_channel, source_type, source_config_json, allowed_signing_fingerprints_json, icon_path, max_releases_per_channel, created_at, updated_at FROM projects WHERE slug = ?",
        slug,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_by_package_name(
    pool: &DbPool,
    package_name: &str,
) -> super::Result<Option<Project>> {
    let row = sqlx::query_as!(
        Project,
        "SELECT id, slug, name, description, package_name, visibility, default_channel, source_type, source_config_json, allowed_signing_fingerprints_json, icon_path, max_releases_per_channel, created_at, updated_at FROM projects WHERE package_name = ?",
        package_name,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub struct CreateProjectInput {
    pub slug: String,
    pub name: String,
    pub package_name: String,
    pub description: String,
    pub max_releases_per_channel: Option<i64>,
}

pub async fn create(pool: &DbPool, input: &CreateProjectInput) -> super::Result<Project> {
    let id = Uuid::now_v7().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query!(
        "INSERT INTO projects (id, slug, name, package_name, description, max_releases_per_channel, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        id,
        input.slug,
        input.name,
        input.package_name,
        input.description,
        input.max_releases_per_channel,
        now,
        now,
    )
    .execute(pool)
    .await?;

    Ok(Project {
        id,
        slug: input.slug.clone(),
        name: input.name.clone(),
        description: input.description.clone(),
        package_name: input.package_name.clone(),
        visibility: "public".to_string(),
        default_channel: "stable".to_string(),
        source_type: "manual".to_string(),
        source_config_json: None,
        allowed_signing_fingerprints_json: None,
        icon_path: None,
        max_releases_per_channel: input.max_releases_per_channel,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub visibility: Option<String>,
    pub default_channel: Option<String>,
    pub source_type: Option<String>,
    pub source_config_json: Option<Option<String>>,
    pub allowed_signing_fingerprints_json: Option<Option<String>>,
    pub icon_path: Option<Option<String>>,
    pub max_releases_per_channel: Option<Option<i64>>,
}

// Dynamic SQL: cannot use query! macros because the SET clauses vary at runtime.
pub async fn update(
    pool: &DbPool,
    id: &str,
    input: &UpdateProjectInput,
) -> super::Result<Option<Project>> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut sets = vec!["updated_at = ?".to_string()];
    let mut binds: Vec<Option<String>> = vec![Some(now)];

    if let Some(ref name) = input.name {
        sets.push("name = ?".to_string());
        binds.push(Some(name.clone()));
    }
    if let Some(ref description) = input.description {
        sets.push("description = ?".to_string());
        binds.push(Some(description.clone()));
    }
    if let Some(ref package_name) = input.package_name {
        sets.push("package_name = ?".to_string());
        binds.push(Some(package_name.clone()));
    }
    if let Some(ref visibility) = input.visibility {
        sets.push("visibility = ?".to_string());
        binds.push(Some(visibility.clone()));
    }
    if let Some(ref default_channel) = input.default_channel {
        sets.push("default_channel = ?".to_string());
        binds.push(Some(default_channel.clone()));
    }
    if let Some(ref source_type) = input.source_type {
        sets.push("source_type = ?".to_string());
        binds.push(Some(source_type.clone()));
    }
    if let Some(ref source_config_json) = input.source_config_json {
        sets.push("source_config_json = ?".to_string());
        binds.push(source_config_json.clone());
    }
    if let Some(ref fingerprints) = input.allowed_signing_fingerprints_json {
        sets.push("allowed_signing_fingerprints_json = ?".to_string());
        binds.push(fingerprints.clone());
    }
    if let Some(ref icon_path) = input.icon_path {
        sets.push("icon_path = ?".to_string());
        binds.push(icon_path.clone());
    }
    if let Some(ref max) = input.max_releases_per_channel {
        sets.push("max_releases_per_channel = ?".to_string());
        binds.push(max.map(|n| n.to_string()));
    }

    let sql = format!("UPDATE projects SET {} WHERE id = ?", sets.join(", "));

    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for b in &binds {
        query = query.bind(b);
    }
    query = query.bind(id);

    let result = query.execute(pool).await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }

    // Return a placeholder; the route handler re-fetches the full project.
    Ok(Some(Project {
        id: id.to_string(),
        slug: String::new(),
        name: String::new(),
        description: String::new(),
        package_name: String::new(),
        visibility: String::new(),
        default_channel: String::new(),
        source_type: String::new(),
        source_config_json: None,
        allowed_signing_fingerprints_json: None,
        icon_path: None,
        max_releases_per_channel: None,
        created_at: String::new(),
        updated_at: String::new(),
    }))
}

pub async fn delete(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!("DELETE FROM projects WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
