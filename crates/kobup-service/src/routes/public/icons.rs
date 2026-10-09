use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};

use crate::AppState;
use crate::repositories;

pub async fn get_icon(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let project = match repositories::projects::get_by_id(&state.pool, &id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    if project.icon_path.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let path = state.storage.icon_path(&id);
    match tokio::fs::read(&path).await {
        Ok(data) => ([(header::CONTENT_TYPE, "image/png")], data).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
