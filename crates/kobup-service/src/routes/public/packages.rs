use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, repositories};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestQuery {
    pub channel: Option<String>,
    pub sdk_int: Option<i64>,
    pub abis: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LatestResponse {
    project_slug: String,
    version_name: String,
    version_code: i64,
    artifact_id: String,
    download_url: String,
    sha256: String,
    size_bytes: i64,
    min_sdk: Option<i64>,
    reason: Option<String>,
}

pub async fn latest_by_package(
    State(state): State<AppState>,
    Path(package_name): Path<String>,
    Query(params): Query<LatestQuery>,
) -> impl IntoResponse {
    let project =
        match repositories::projects::get_by_package_name(&state.pool, &package_name).await {
            Ok(Some(p)) => p,
            Ok(None) => return (StatusCode::NOT_FOUND, "package not found").into_response(),
            Err(e) => {
                tracing::error!(error = %e, "failed to find project by package_name");
                return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
            }
        };

    let channel_name = params.channel.as_deref().unwrap_or("stable");

    // Find the channel
    let channels = match repositories::channels::list_by_project(&state.pool, &project.id).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to list channels");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let channel = match channels.iter().find(|c| c.name == channel_name) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "channel not found").into_response(),
    };

    let release =
        match repositories::releases::get_latest_published(&state.pool, &project.id, &channel.id)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => return (StatusCode::NOT_FOUND, "no published release").into_response(),
            Err(e) => {
                tracing::error!(error = %e, "failed to get latest release");
                return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
            }
        };

    let abis: Vec<String> = params
        .abis
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let artifact = match repositories::artifacts::get_best_for_release(
        &state.pool,
        &release.id,
        params.sdk_int,
        &abis,
    )
    .await
    {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, "no compatible artifact").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to find artifact");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let download_url = format!(
        "{}/api/v1/artifacts/{}/download",
        state.config.base_url, artifact.id
    );

    Json(LatestResponse {
        project_slug: project.slug,
        version_name: release.version_name,
        version_code: release.version_code,
        artifact_id: artifact.id,
        download_url,
        sha256: artifact.sha256,
        size_bytes: artifact.size_bytes,
        min_sdk: artifact.min_sdk,
        reason: None,
    })
    .into_response()
}
