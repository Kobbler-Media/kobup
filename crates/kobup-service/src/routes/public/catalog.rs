use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogQuery {
    pub is_tv: Option<bool>,
    pub search: Option<String>,
    pub installed_package_names: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct CatalogRow {
    id: String,
    slug: String,
    name: String,
    description: String,
    package_name: String,
    version_name: Option<String>,
    version_code: Option<i64>,
    channel_name: Option<String>,
    artifact_id: Option<String>,
    artifact_sha256: Option<String>,
    artifact_size_bytes: Option<i64>,
    icon_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogItem {
    id: String,
    slug: String,
    name: String,
    description: String,
    package_name: String,
    latest_version: Option<LatestVersion>,
    artifact_id: Option<String>,
    download_url: Option<String>,
    sha256: Option<String>,
    size_bytes: Option<i64>,
    icon_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LatestVersion {
    version_name: String,
    version_code: i64,
    channel: String,
}

pub async fn catalog(
    State(state): State<AppState>,
    Query(params): Query<CatalogQuery>,
) -> impl IntoResponse {
    // Dynamic SQL: uses runtime sqlx::query_as() because the WHERE clauses
    // vary based on search/filter parameters. Cannot use compile-time query! macros.
    //
    // When a channel is supplied the latest-release subquery is restricted to
    // releases in a channel of that name (channels are per-project), so devices
    // on a non-stable channel only ever see that channel's releases. When no
    // channel is supplied the newest published release across all channels wins.
    let mut bind_values: Vec<String> = Vec::new();
    let (inner_channel_filter, outer_channel_filter) = if params.channel.is_some() {
        (
            " AND channel_id IN (SELECT id FROM channels WHERE name = ?)",
            " AND r1.channel_id IN (SELECT id FROM channels WHERE name = ?)",
        )
    } else {
        ("", "")
    };
    if let Some(ref channel) = params.channel {
        // Two placeholders (inner MAX subquery + outer release filter), in
        // textual order, so push the channel value twice up front.
        bind_values.push(channel.clone());
        bind_values.push(channel.clone());
    }

    let mut sql = format!(
        "SELECT p.id, p.slug, p.name, p.description, p.package_name, \
         r.version_name, r.version_code, c.name as channel_name, \
         a.id as artifact_id, a.sha256 as artifact_sha256, a.size_bytes as artifact_size_bytes, \
         p.icon_path \
         FROM projects p \
         LEFT JOIN (\
             SELECT r1.* FROM releases r1 \
             INNER JOIN (\
                 SELECT project_id, MAX(version_code) as max_vc \
                 FROM releases WHERE published_at IS NOT NULL{inner_channel_filter} \
                 GROUP BY project_id\
             ) r2 ON r1.project_id = r2.project_id AND r1.version_code = r2.max_vc \
             WHERE r1.published_at IS NOT NULL{outer_channel_filter}\
         ) r ON r.project_id = p.id \
         LEFT JOIN channels c ON c.id = r.channel_id \
         LEFT JOIN (\
             SELECT a1.* FROM artifacts a1 \
             INNER JOIN (\
                 SELECT release_id, MIN(CASE WHEN is_universal THEN 0 ELSE 1 END) as pref, \
                        MIN(id) as min_id \
                 FROM artifacts GROUP BY release_id\
             ) a2 ON a1.release_id = a2.release_id \
             AND a1.id = (\
                 SELECT id FROM artifacts \
                 WHERE release_id = a1.release_id \
                 ORDER BY is_universal DESC, id ASC LIMIT 1\
             )\
         ) a ON a.release_id = r.id \
         WHERE p.visibility = 'public'"
    );

    if let Some(ref search) = params.search
        && !search.is_empty()
    {
        sql.push_str(" AND (p.name LIKE ? OR p.slug LIKE ? OR p.package_name LIKE ?)");
        let pattern = format!("%{search}%");
        bind_values.push(pattern.clone());
        bind_values.push(pattern.clone());
        bind_values.push(pattern);
    }

    if let Some(ref installed) = params.installed_package_names
        && !installed.is_empty()
    {
        let names: Vec<&str> = installed
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !names.is_empty() {
            let placeholders: Vec<&str> = names.iter().map(|_| "?").collect();
            sql.push_str(&format!(
                " AND p.package_name IN ({})",
                placeholders.join(",")
            ));
            for name in names {
                bind_values.push(name.to_string());
            }
        }
    }

    sql.push_str(" ORDER BY p.name ASC");

    let mut query = sqlx::query_as::<_, CatalogRow>(sqlx::AssertSqlSafe(sql));
    for val in &bind_values {
        query = query.bind(val);
    }

    match query.fetch_all(&state.pool).await {
        Ok(rows) => {
            let items: Vec<CatalogItem> = rows
                .into_iter()
                .map(|row| {
                    let download_url = row.artifact_id.as_ref().map(|aid| {
                        format!(
                            "{}/api/v1/artifacts/{}/download",
                            state.config.base_url, aid
                        )
                    });
                    let icon_url = row.icon_path.as_ref().map(|_| {
                        format!("{}/api/v1/projects/{}/icon", state.config.base_url, row.id)
                    });
                    CatalogItem {
                        id: row.id,
                        slug: row.slug,
                        name: row.name,
                        description: row.description,
                        package_name: row.package_name,
                        latest_version: row.version_name.map(|vn| LatestVersion {
                            version_name: vn,
                            version_code: row.version_code.unwrap_or(0),
                            channel: row.channel_name.unwrap_or_else(|| "stable".to_string()),
                        }),
                        artifact_id: row.artifact_id,
                        download_url,
                        sha256: row.artifact_sha256,
                        size_bytes: row.artifact_size_bytes,
                        icon_url,
                    }
                })
                .collect();
            Json(items).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch catalog");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch catalog").into_response()
        }
    }
}
