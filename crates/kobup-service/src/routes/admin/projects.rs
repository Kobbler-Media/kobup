use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::routes::{AuthUser, SessionUser};
use crate::{AppState, repositories};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectResponse {
    id: String,
    slug: String,
    name: String,
    description: String,
    package_name: String,
    visibility: String,
    default_channel: String,
    source_type: String,
    source_config_json: Option<String>,
    allowed_signing_fingerprints_json: Option<String>,
    icon_url: Option<String>,
    max_releases_per_channel: Option<i64>,
    created_at: String,
    updated_at: String,
}

impl From<repositories::projects::Project> for ProjectResponse {
    fn from(p: repositories::projects::Project) -> Self {
        Self {
            id: p.id.clone(),
            slug: p.slug,
            name: p.name,
            description: p.description,
            package_name: p.package_name,
            visibility: p.visibility,
            default_channel: p.default_channel,
            source_type: p.source_type,
            source_config_json: p.source_config_json,
            allowed_signing_fingerprints_json: p.allowed_signing_fingerprints_json,
            icon_url: p
                .icon_path
                .as_ref()
                .map(|_| format!("/api/v1/projects/{}/icon", p.id)),
            max_releases_per_channel: p.max_releases_per_channel,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDetailResponse {
    #[serde(flatten)]
    project: ProjectResponse,
    channels: Vec<ChannelResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChannelResponse {
    id: String,
    project_id: String,
    name: String,
    is_default: bool,
    created_at: String,
}

impl From<repositories::channels::Channel> for ChannelResponse {
    fn from(c: repositories::channels::Channel) -> Self {
        Self {
            id: c.id,
            project_id: c.project_id,
            name: c.name,
            is_default: c.is_default,
            created_at: c.created_at,
        }
    }
}

pub async fn list(State(state): State<AppState>, _auth: SessionUser) -> impl IntoResponse {
    match repositories::projects::list(&state.pool).await {
        Ok(projects) => {
            let resp: Vec<ProjectResponse> = projects.into_iter().map(Into::into).collect();
            Json(resp).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to list projects");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to list projects").into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub slug: String,
    pub name: String,
    pub package_name: Option<String>,
    pub description: Option<String>,
    pub max_releases_per_channel: Option<i64>,
}

pub async fn create(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    // Idempotent by slug
    if let Ok(Some(existing)) =
        repositories::projects::get_by_slug(&state.pool, &payload.slug).await
    {
        let channels = repositories::channels::list_by_project(&state.pool, &existing.id)
            .await
            .unwrap_or_default();
        return (
            StatusCode::OK,
            Json(ProjectDetailResponse {
                project: existing.into(),
                channels: channels.into_iter().map(Into::into).collect(),
            }),
        )
            .into_response();
    }

    let input = repositories::projects::CreateProjectInput {
        slug: payload.slug,
        name: payload.name,
        package_name: payload.package_name.unwrap_or_default(),
        description: payload.description.unwrap_or_default(),
        max_releases_per_channel: payload.max_releases_per_channel,
    };

    let project = match repositories::projects::create(&state.pool, &input).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to create project");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create project",
            )
                .into_response();
        }
    };

    // Auto-create "stable" channel
    let channel =
        match repositories::channels::create(&state.pool, &project.id, "stable", true).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "failed to create default channel");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to create project",
                )
                    .into_response();
            }
        };

    (
        StatusCode::CREATED,
        Json(ProjectDetailResponse {
            project: project.into(),
            channels: vec![channel.into()],
        }),
    )
        .into_response()
}

pub async fn get(
    State(state): State<AppState>,
    _auth: SessionUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let project = match repositories::projects::get_by_id(&state.pool, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to get project").into_response();
        }
    };

    let channels = repositories::channels::list_by_project(&state.pool, &project.id)
        .await
        .unwrap_or_default();

    Json(ProjectDetailResponse {
        project: project.into(),
        channels: channels.into_iter().map(Into::into).collect(),
    })
    .into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub visibility: Option<String>,
    pub default_channel: Option<String>,
    pub source_type: Option<String>,
    pub source_config_json: Option<Option<String>>,
    pub allowed_signing_fingerprints_json: Option<Option<String>>,
    pub max_releases_per_channel: Option<Option<i64>>,
}

pub async fn update(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProjectRequest>,
) -> impl IntoResponse {
    let input = repositories::projects::UpdateProjectInput {
        name: payload.name,
        description: payload.description,
        package_name: payload.package_name,
        visibility: payload.visibility,
        default_channel: payload.default_channel,
        source_type: payload.source_type,
        source_config_json: payload.source_config_json,
        allowed_signing_fingerprints_json: payload.allowed_signing_fingerprints_json,
        icon_path: None,
        max_releases_per_channel: payload.max_releases_per_channel,
    };

    match repositories::projects::update(&state.pool, &id, &input).await {
        Ok(Some(_)) => {
            // Re-fetch the updated project
            match repositories::projects::get_by_id(&state.pool, &id).await {
                Ok(Some(project)) => {
                    let channels =
                        repositories::channels::list_by_project(&state.pool, &project.id)
                            .await
                            .unwrap_or_default();
                    Json(ProjectDetailResponse {
                        project: project.into(),
                        channels: channels.into_iter().map(Into::into).collect(),
                    })
                    .into_response()
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to fetch updated project",
                )
                    .into_response(),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to update project");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update project",
            )
                .into_response()
        }
    }
}

pub async fn delete_project(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Collect all artifacts for file cleanup before cascade-deleting
    let release_artifacts = match repositories::releases::list_by_project(&state.pool, &id).await {
        Ok(releases) => {
            let mut all = Vec::new();
            for release in &releases {
                if let Ok(artifacts) =
                    repositories::artifacts::list_by_release(&state.pool, &release.id).await
                {
                    all.extend(artifacts);
                }
            }
            all
        }
        Err(_) => Vec::new(),
    };

    match repositories::projects::delete(&state.pool, &id).await {
        Ok(true) => {
            // Clean up artifact files from disk — by their recorded path, not a
            // path rebuilt from the row ID (existing rows predate the fix that
            // made those match).
            for artifact in &release_artifacts {
                if let Err(e) = state
                    .storage
                    .delete_artifact_file(&artifact.file_path_or_object_key)
                    .await
                {
                    tracing::warn!(artifact_id = %artifact.id, error = %e, "failed to delete artifact file");
                }
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to delete project");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to delete project",
            )
                .into_response()
        }
    }
}

const MAX_ICON_BYTES: usize = 256 * 1024; // 256 KB

pub async fn upload_icon(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<String>,
    body: Bytes,
) -> impl IntoResponse {
    if body.len() > MAX_ICON_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE, "icon must be 256KB or less").into_response();
    }

    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty body").into_response();
    }

    // Verify project exists
    match repositories::projects::get_by_id(&state.pool, &id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    }

    // Store icon file
    let icon_path = match state.storage.store_icon(&id, &body).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to store icon");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to store icon").into_response();
        }
    };

    // Update project record
    let input = repositories::projects::UpdateProjectInput {
        name: None,
        description: None,
        package_name: None,
        visibility: None,
        default_channel: None,
        source_type: None,
        source_config_json: None,
        allowed_signing_fingerprints_json: None,
        icon_path: Some(Some(icon_path)),
        max_releases_per_channel: None,
    };

    if let Err(e) = repositories::projects::update(&state.pool, &id, &input).await {
        tracing::error!(error = %e, "failed to update project icon_path");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update project",
        )
            .into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}
