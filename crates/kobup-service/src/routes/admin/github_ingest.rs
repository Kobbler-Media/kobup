use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::routes::AuthUser;
use crate::{AppState, repositories};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubIngestRequest {
    pub owner: String,
    pub repo: String,
    pub release_asset_pattern: Option<String>,
    pub auth_mode: Option<String>,
}

pub async fn configure(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(project_id): Path<String>,
    Json(payload): Json<GithubIngestRequest>,
) -> impl IntoResponse {
    // Check project exists
    match repositories::projects::get_by_id(&state.pool, &project_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    }

    let config_json = serde_json::json!({
        "owner": payload.owner,
        "repo": payload.repo,
        "releaseAssetPattern": payload.release_asset_pattern,
        "authMode": payload.auth_mode,
    })
    .to_string();

    let input = repositories::projects::UpdateProjectInput {
        name: None,
        description: None,
        package_name: None,
        visibility: None,
        default_channel: None,
        source_type: Some("github".to_string()),
        source_config_json: Some(Some(config_json)),
        allowed_signing_fingerprints_json: None,
        icon_path: None,
        max_releases_per_channel: None,
    };

    match repositories::projects::update(&state.pool, &project_id, &input).await {
        Ok(Some(_)) => StatusCode::OK.into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to update project source config");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to configure github ingest",
            )
                .into_response()
        }
    }
}
