use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use tracing::warn;

use crate::AppState;
use crate::repositories::device_push_tokens;
use crate::routes::AuthUser;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushRefreshRequest {
    pub project_slug: Option<String>,
    pub channel: Option<String>,
    pub broadcast: Option<bool>,
}

pub async fn push_refresh(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(payload): Json<PushRefreshRequest>,
) -> impl IntoResponse {
    let tokens = match device_push_tokens::list_all_tokens(&state.pool).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "failed to list push tokens");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    if tokens.is_empty() {
        warn!("no devices have registered a push token, skipping push refresh");
        return StatusCode::OK;
    }

    let project_slug = payload.project_slug.as_deref().unwrap_or("*");
    let channel = payload.channel.as_deref().unwrap_or("stable");

    match state.fcm {
        Some(ref fcm) => {
            if let Err(e) = fcm.send_refresh(&tokens, project_slug, channel).await {
                tracing::error!(error = %e, "FCM send_refresh failed");
            }
        }
        None => {
            warn!("FCM not configured, skipping push refresh");
        }
    }

    StatusCode::OK
}
