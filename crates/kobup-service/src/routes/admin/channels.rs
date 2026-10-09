use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::routes::AuthUser;
use crate::{AppState, repositories};

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(default)]
    pub is_default: bool,
}

pub async fn create(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(project_id): Path<String>,
    Json(payload): Json<CreateChannelRequest>,
) -> impl IntoResponse {
    // Verify project exists
    match repositories::projects::get_by_id(&state.pool, &project_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to check project");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create channel",
            )
                .into_response();
        }
    }

    match repositories::channels::create(
        &state.pool,
        &project_id,
        &payload.name,
        payload.is_default,
    )
    .await
    {
        Ok(channel) => (StatusCode::CREATED, Json(ChannelResponse::from(channel))).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to create channel");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create channel",
            )
                .into_response()
        }
    }
}

pub async fn delete_channel(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((_project_id, id)): Path<(String, String)>,
) -> impl IntoResponse {
    match repositories::channels::delete(&state.pool, &id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "channel not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to delete channel");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to delete channel",
            )
                .into_response()
        }
    }
}
