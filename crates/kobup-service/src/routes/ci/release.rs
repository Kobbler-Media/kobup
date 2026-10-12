use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::AppState;
use crate::repositories::projects;
use crate::routes::{CiAuth, release_upload};

pub async fn upload(
    State(state): State<AppState>,
    ci_auth: CiAuth,
    Path(slug): Path<String>,
    multipart: Multipart,
) -> Response {
    let project = match projects::get_by_slug(&state.pool, &slug).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };
    if ci_auth
        .project_id
        .as_ref()
        .is_some_and(|scoped| *scoped != project.id)
    {
        return (
            StatusCode::FORBIDDEN,
            "CI token is not authorized for this project",
        )
            .into_response();
    }
    release_upload::handle(&state, &project, multipart, "ci").await
}
