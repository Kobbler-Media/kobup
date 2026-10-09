use axum::{
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use tokio_util::io::ReaderStream;

use crate::{AppState, repositories};

pub async fn download(
    State(state): State<AppState>,
    Path(artifact_id): Path<String>,
) -> impl IntoResponse {
    let artifact = match repositories::artifacts::get_by_id(&state.pool, &artifact_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, "artifact not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get artifact");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    let file = match tokio::fs::File::open(&artifact.file_path_or_object_key).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(error = %e, path = %artifact.file_path_or_object_key, "failed to open artifact file");
            return (StatusCode::NOT_FOUND, "artifact file not found").into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_disposition = format!("attachment; filename=\"{}\"", artifact.file_name);

    (
        [
            (header::CONTENT_TYPE, artifact.mime_type),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        body,
    )
        .into_response()
}
