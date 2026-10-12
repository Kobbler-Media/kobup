use axum::{
    Json,
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::AppState;
use crate::dex_metadata;
use crate::release_ingest::{self, ApkFile, IngestError, NewRelease};
use crate::repositories::projects::Project;
use crate::routes::admin::releases::ArtifactResponse;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    release: ReleaseResponse,
    // Pre-split clients read the single `artifact`.
    artifact: ArtifactResponse,
    artifacts: Vec<ArtifactResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseResponse {
    id: String,
    project_id: String,
    channel_id: String,
    version_code: i64,
    version_name: String,
    release_notes: Option<String>,
    source_type: String,
    published_at: Option<String>,
    created_at: String,
}

pub async fn handle(
    state: &AppState,
    project: &Project,
    multipart: Multipart,
    source_type: &str,
) -> Response {
    let input = match read_form(multipart, source_type).await {
        Ok(input) => input,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };
    match release_ingest::ingest(&state.pool, &state.storage, project, input).await {
        Ok((release, artifacts)) => {
            let primary = artifacts
                .iter()
                .find(|a| a.is_universal)
                .unwrap_or(&artifacts[0])
                .clone();
            (
                StatusCode::CREATED,
                Json(UploadResponse {
                    release: ReleaseResponse {
                        id: release.id,
                        project_id: release.project_id,
                        channel_id: release.channel_id,
                        version_code: release.version_code,
                        version_name: release.version_name,
                        release_notes: release.release_notes,
                        source_type: release.source_type,
                        published_at: release.published_at,
                        created_at: release.created_at,
                    },
                    artifact: primary.into(),
                    artifacts: artifacts.into_iter().map(Into::into).collect(),
                }),
            )
                .into_response()
        }
        Err(IngestError::Invalid(msg)) => (StatusCode::BAD_REQUEST, msg).into_response(),
        Err(IngestError::Conflict(msg)) => (StatusCode::CONFLICT, msg).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to ingest release");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to store release").into_response()
        }
    }
}

async fn read_form(mut multipart: Multipart, source_type: &str) -> Result<NewRelease, String> {
    let mut input = NewRelease {
        apks: Vec::new(),
        channel_name: "stable".into(),
        release_notes: None,
        publish: false,
        version_code: None,
        version_name: None,
        package_name: None,
        profiles: Vec::new(),
        source_type: source_type.into(),
        source_ref: None,
    };

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                tracing::error!(error = %e, "malformed multipart body");
                return Err("malformed multipart body".into());
            }
        };
        let name = field.name().unwrap_or("").to_string();
        if name == "apk" {
            let file_name = field.file_name().unwrap_or("upload.apk").to_string();
            let bytes = field.bytes().await.map_err(|e| {
                tracing::error!(error = %e, "failed to read APK field");
                "failed to read APK file".to_string()
            })?;
            input.apks.push(ApkFile {
                file_name,
                bytes: bytes.to_vec(),
            });
            continue;
        }
        if let Some(min_api) = dex_metadata::field_min_api(&name) {
            let bytes = field.bytes().await.map_err(|e| {
                tracing::error!(error = %e, "failed to read dex metadata field");
                "failed to read dex metadata".to_string()
            })?;
            input.profiles.push((min_api, bytes.to_vec()));
            continue;
        }
        let Some(text) = field.text().await.ok().filter(|t| !t.is_empty()) else {
            continue;
        };
        match name.as_str() {
            "channel" => input.channel_name = text,
            "release_notes" => input.release_notes = Some(text),
            "publish" => input.publish = text == "true" || text == "1",
            "version_code" => {
                input.version_code = Some(
                    text.parse()
                        .map_err(|_| format!("invalid version_code: '{text}'"))?,
                )
            }
            "version_name" => input.version_name = Some(text),
            "package_name" => input.package_name = Some(text),
            _ => {}
        }
    }
    Ok(input)
}
