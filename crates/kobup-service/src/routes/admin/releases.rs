use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;

use crate::AppState;
use crate::repositories::{artifacts, releases};
use crate::routes::AuthUser;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseResponse {
    id: String,
    project_id: String,
    channel_id: String,
    version_code: i64,
    version_name: String,
    release_notes: Option<String>,
    source_type: String,
    source_ref: Option<String>,
    published_at: Option<String>,
    created_at: String,
    artifacts: Vec<ArtifactResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactResponse {
    id: String,
    release_id: String,
    file_name: String,
    package_name: String,
    sha256: String,
    size_bytes: i64,
    version_code: i64,
    version_name: String,
    supported_abis: Vec<String>,
    is_universal: bool,
    created_at: String,
}

impl From<artifacts::Artifact> for ArtifactResponse {
    fn from(a: artifacts::Artifact) -> Self {
        let abis: Vec<String> = serde_json::from_str(&a.supported_abis_json).unwrap_or_default();
        Self {
            id: a.id,
            release_id: a.release_id,
            file_name: a.file_name,
            package_name: a.package_name,
            sha256: a.sha256,
            size_bytes: a.size_bytes,
            version_code: a.version_code,
            version_name: a.version_name,
            supported_abis: abis,
            is_universal: a.is_universal,
            created_at: a.created_at,
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let release_list = match releases::list_by_project(&state.pool, &project_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to list releases");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to list releases").into_response();
        }
    };

    let mut responses = Vec::with_capacity(release_list.len());
    for release in release_list {
        let arts = artifacts::list_by_release(&state.pool, &release.id)
            .await
            .unwrap_or_default();
        responses.push(ReleaseResponse {
            id: release.id,
            project_id: release.project_id,
            channel_id: release.channel_id,
            version_code: release.version_code,
            version_name: release.version_name,
            release_notes: release.release_notes,
            source_type: release.source_type,
            source_ref: release.source_ref,
            published_at: release.published_at,
            created_at: release.created_at,
            artifacts: arts.into_iter().map(Into::into).collect(),
        });
    }

    Json(responses).into_response()
}

pub async fn publish(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match releases::publish(&state.pool, &id).await {
        Ok(true) => match releases::get_by_id(&state.pool, &id).await {
            Ok(Some(release)) => {
                let arts = artifacts::list_by_release(&state.pool, &release.id)
                    .await
                    .unwrap_or_default();
                Json(ReleaseResponse {
                    id: release.id,
                    project_id: release.project_id,
                    channel_id: release.channel_id,
                    version_code: release.version_code,
                    version_name: release.version_name,
                    release_notes: release.release_notes,
                    source_type: release.source_type,
                    source_ref: release.source_ref,
                    published_at: release.published_at,
                    created_at: release.created_at,
                    artifacts: arts.into_iter().map(Into::into).collect(),
                })
                .into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch release").into_response(),
        },
        Ok(false) => {
            // Check if release exists at all or was already published
            match releases::get_by_id(&state.pool, &id).await {
                Ok(Some(r)) if r.published_at.is_some() => {
                    (StatusCode::CONFLICT, "release already published").into_response()
                }
                Ok(Some(_)) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "failed to publish").into_response()
                }
                _ => (StatusCode::NOT_FOUND, "release not found").into_response(),
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to publish release");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to publish release",
            )
                .into_response()
        }
    }
}

pub async fn delete_release(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Get artifacts first for storage cleanup
    let release_artifacts = match artifacts::list_by_release(&state.pool, &id).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "failed to list artifacts for release");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    // Delete release (cascades to artifacts via FK)
    match releases::delete(&state.pool, &id).await {
        Ok(true) => {
            // Clean up stored files — by their recorded path, not a path rebuilt
            // from the row ID (existing rows predate the fix that made those match).
            for artifact in &release_artifacts {
                if let Err(e) = state
                    .storage
                    .delete_artifact_file(&artifact.file_path_or_object_key)
                    .await
                {
                    tracing::warn!(artifact_id = %artifact.id, error = %e, "failed to delete artifact files");
                }
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "release not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to delete release");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to delete release",
            )
                .into_response()
        }
    }
}
