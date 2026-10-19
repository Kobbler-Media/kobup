use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::warn;

use crate::AppState;
use crate::repositories::{device_push_tokens, projects};
use crate::routes::CiAuth;

pub async fn push_refresh(
    State(state): State<AppState>,
    ci_auth: CiAuth,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let project = match projects::get_by_slug(&state.pool, &slug).await {
        Ok(Some(p)) => p,
        Ok(None) => return StatusCode::NOT_FOUND,
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    if let Some(ref token_project_id) = ci_auth.project_id
        && *token_project_id != project.id
    {
        return StatusCode::FORBIDDEN;
    }

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

    match state.fcm {
        Some(ref fcm) => {
            if let Err(e) = fcm.send_refresh(&tokens, &slug, "stable").await {
                tracing::error!(error = %e, "FCM send_refresh failed");
            }
        }
        None => {
            warn!("FCM not configured, skipping push refresh");
        }
    }

    StatusCode::OK
}
