use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::AppState;
use crate::repositories::projects;
use crate::routes::{AuthUser, release_upload};

pub async fn upload(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(project_id): Path<String>,
    multipart: Multipart,
) -> Response {
    let project = match projects::get_by_id(&state.pool, &project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };
    release_upload::handle(&state, &project, multipart, "upload").await
}
