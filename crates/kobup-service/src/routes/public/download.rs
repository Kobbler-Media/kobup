use axum::{
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use tokio_util::io::ReaderStream;

use crate::{AppState, repositories};

pub async fn download(State(state): State<AppState>, Path(artifact_id): Path<String>) -> Response {
    let artifact = match repositories::artifacts::get_by_id(&state.pool, &artifact_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, "artifact not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get artifact");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    stream_file(
        &artifact.file_path_or_object_key,
        artifact.mime_type,
        &artifact.file_name,
    )
    .await
}

pub async fn dex_metadata(
    State(state): State<AppState>,
    Path((artifact_id, min_api)): Path<(String, i64)>,
) -> Response {
    let dm = match repositories::dex_metadata::get(&state.pool, &artifact_id, min_api).await {
        Ok(Some(dm)) => dm,
        Ok(None) => return (StatusCode::NOT_FOUND, "dex metadata not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get dex metadata");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    stream_file(
        &dm.file_path,
        "application/octet-stream".to_string(),
        &format!("{min_api}.dm"),
    )
    .await
}

async fn stream_file(path: &str, mime_type: String, file_name: &str) -> Response {
    let file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(error = %e, path = %path, "failed to open artifact file");
            return (StatusCode::NOT_FOUND, "artifact file not found").into_response();
        }
    };

    (
        [
            (header::CONTENT_TYPE, mime_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{file_name}\""),
            ),
        ],
        Body::from_stream(ReaderStream::new(file)),
    )
        .into_response()
}
