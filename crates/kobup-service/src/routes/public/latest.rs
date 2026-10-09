use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::repositories::{self, projects::Project};

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
    dex_metadata: Option<DexMetadataRef>,
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DexMetadataRef {
    min_api: i64,
    download_url: String,
    sha256: String,
    size_bytes: i64,
}

pub async fn latest_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<LatestQuery>,
) -> Response {
    match repositories::projects::get_by_slug(&state.pool, &slug).await {
        Ok(Some(project)) => latest(&state, project, params).await,
        Ok(None) => (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}

pub async fn latest_by_package(
    State(state): State<AppState>,
    Path(package_name): Path<String>,
    Query(params): Query<LatestQuery>,
) -> Response {
    match repositories::projects::get_by_package_name(&state.pool, &package_name).await {
        Ok(Some(project)) => latest(&state, project, params).await,
        Ok(None) => (StatusCode::NOT_FOUND, "package not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to find project by package_name");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}

async fn latest(state: &AppState, project: Project, params: LatestQuery) -> Response {
    let channel_name = params.channel.as_deref().unwrap_or("stable");

    let channels = match repositories::channels::list_by_project(&state.pool, &project.id).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to list channels");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let Some(channel) = channels.iter().find(|c| c.name == channel_name) else {
        return (StatusCode::NOT_FOUND, "channel not found").into_response();
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

    // Without the device's API level there is no telling which profile format it reads.
    let dex_metadata = match params.sdk_int {
        Some(sdk) => {
            match repositories::dex_metadata::get_for_sdk(&state.pool, &artifact.id, sdk).await {
                Ok(dm) => dm,
                Err(e) => {
                    tracing::error!(error = %e, "failed to look up dex metadata");
                    return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
                }
            }
        }
        None => None,
    };

    let base_url = &state.config.base_url;
    Json(LatestResponse {
        project_slug: project.slug,
        version_name: release.version_name,
        version_code: release.version_code,
        download_url: format!("{base_url}/api/v1/artifacts/{}/download", artifact.id),
        dex_metadata: dex_metadata.map(|dm| DexMetadataRef {
            download_url: format!(
                "{base_url}/api/v1/artifacts/{}/dm/{}",
                artifact.id, dm.min_api
            ),
            min_api: dm.min_api,
            sha256: dm.sha256,
            size_bytes: dm.size_bytes,
        }),
        artifact_id: artifact.id,
        sha256: artifact.sha256,
        size_bytes: artifact.size_bytes,
        min_sdk: artifact.min_sdk,
        reason: None,
    })
    .into_response()
}
