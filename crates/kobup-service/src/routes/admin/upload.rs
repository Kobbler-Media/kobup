use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;

use crate::AppState;
use crate::apk;
use crate::dex_metadata;
use crate::repositories::{artifacts, channels, projects, releases};
use crate::routes::AuthUser;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    release: ReleaseResponse,
    artifact: ArtifactResponse,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactResponse {
    id: String,
    release_id: String,
    file_name: String,
    package_name: String,
    sha256: String,
    size_bytes: i64,
    version_code: i64,
    version_name: String,
    supported_abis: Vec<String>,
    is_universal: bool,
    created_at: String,
}

pub async fn upload(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(project_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Parse multipart fields
    let mut apk_bytes: Option<(String, Vec<u8>)> = None;
    let mut channel_name = "stable".to_string();
    let mut release_notes: Option<String> = None;
    let mut publish = false;
    let mut override_version_code: Option<String> = None;
    let mut override_version_name: Option<String> = None;
    let mut override_package_name: Option<String> = None;
    let mut profiles: Vec<(i64, Vec<u8>)> = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => {
                tracing::error!(error = %e, "malformed multipart body");
                return (StatusCode::BAD_REQUEST, "malformed multipart body").into_response();
            }
        };
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "apk" => {
                let filename = field.file_name().unwrap_or("upload.apk").to_string();
                match field.bytes().await {
                    Ok(bytes) => apk_bytes = Some((filename, bytes.to_vec())),
                    Err(e) => {
                        tracing::error!(error = %e, "failed to read APK field");
                        return (StatusCode::BAD_REQUEST, "failed to read APK file")
                            .into_response();
                    }
                }
            }
            "channel" => {
                if let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    channel_name = text;
                }
            }
            "release_notes" => {
                if let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    release_notes = Some(text);
                }
            }
            "publish" => {
                if let Ok(text) = field.text().await {
                    publish = text == "true" || text == "1";
                }
            }
            "version_code" => {
                if let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    override_version_code = Some(text);
                }
            }
            "version_name" => {
                if let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    override_version_name = Some(text);
                }
            }
            "package_name" => {
                if let Ok(text) = field.text().await
                    && !text.is_empty()
                {
                    override_package_name = Some(text);
                }
            }
            other => {
                if let Some(min_api) = dex_metadata::field_min_api(other) {
                    match field.bytes().await {
                        Ok(bytes) => profiles.push((min_api, bytes.to_vec())),
                        Err(e) => {
                            tracing::error!(error = %e, "failed to read dex metadata field");
                            return (StatusCode::BAD_REQUEST, "failed to read dex metadata")
                                .into_response();
                        }
                    }
                }
            }
        }
    }

    let (filename, bytes) = match apk_bytes {
        Some(b) => b,
        None => return (StatusCode::BAD_REQUEST, "missing apk field").into_response(),
    };

    // Verify project exists
    let project = match projects::get_by_id(&state.pool, &project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get project");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    // Extract APK metadata (best-effort — ZIP parse failure is the only hard error)
    let metadata = match apk::extract_metadata(&bytes) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "failed to extract APK metadata");
            return (StatusCode::BAD_REQUEST, format!("invalid APK: {e}")).into_response();
        }
    };

    // Resolve final values: form overrides take precedence over APK metadata
    let package_name = override_package_name
        .or(metadata.package_name)
        .or_else(|| {
            if !project.package_name.is_empty() {
                Some(project.package_name.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let version_code: i64 = match override_version_code {
        Some(ref v) => match v.parse::<i64>() {
            Ok(vc) => vc,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("invalid version_code: '{v}'"),
                )
                    .into_response();
            }
        },
        None => match metadata.version_code {
            Some(vc) => vc as i64,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "could not extract version_code from APK; specify version_code in the upload form".to_string(),
                )
                    .into_response();
            }
        },
    };

    let version_name = override_version_name
        .or(metadata.version_name)
        .unwrap_or_else(|| "unknown".to_string());

    // Validate package name
    if !project.package_name.is_empty()
        && !package_name.is_empty()
        && project.package_name != package_name
    {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "package name mismatch: project expects '{}', got '{}'",
                project.package_name, package_name
            ),
        )
            .into_response();
    }

    // Auto-fill package_name if not set
    if project.package_name.is_empty() && !package_name.is_empty() {
        let update_input = projects::UpdateProjectInput {
            name: None,
            description: None,
            package_name: Some(package_name.clone()),
            visibility: None,
            default_channel: None,
            source_type: None,
            source_config_json: None,
            allowed_signing_fingerprints_json: None,
            icon_path: None,
            max_releases_per_channel: None,
        };
        if let Err(e) = projects::update(&state.pool, &project_id, &update_input).await {
            tracing::error!(error = %e, "failed to auto-fill package_name");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    }

    // Look up channel
    let channel = match channels::get_by_name(&state.pool, &project_id, &channel_name).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("channel '{channel_name}' not found for this project"),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to look up channel");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };

    // Check for duplicate version_code
    match releases::exists_version_code(&state.pool, &project_id, &channel.id, version_code).await {
        Ok(true) => {
            return (
                StatusCode::CONFLICT,
                format!(
                    "version_code {} already exists in channel '{}'",
                    version_code, channel_name
                ),
            )
                .into_response();
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!(error = %e, "failed to check version_code");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    }

    // Compute SHA256
    let sha256 = apk::compute_sha256(&bytes);
    let size_bytes = bytes.len() as i64;

    // Create release
    let release = match releases::create(
        &state.pool,
        releases::CreateRelease {
            project_id: project_id.clone(),
            channel_id: channel.id.clone(),
            version_code,
            version_name: version_name.clone(),
            release_notes,
            source_type: "upload".to_string(),
            source_ref: None,
        },
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to create release");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create release",
            )
                .into_response();
        }
    };

    // Generate artifact ID and store file
    let artifact_id = uuid::Uuid::now_v7().to_string();
    let file_path = match state
        .storage
        .store_artifact(&artifact_id, &filename, &bytes)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            let _ = releases::delete(&state.pool, &release.id).await;
            if e.kind() == std::io::ErrorKind::InvalidInput {
                return (StatusCode::BAD_REQUEST, "invalid filename").into_response();
            }
            tracing::error!(error = %e, "failed to store APK file");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to store file").into_response();
        }
    };

    let is_universal = metadata.abis.is_empty();
    let supported_abis_json = serde_json::to_string(&metadata.abis).unwrap_or_else(|_| "[]".into());

    // Create artifact record, using the same ID already used for the storage folder.
    let artifact_input = artifacts::CreateArtifact {
        id: artifact_id.clone(),
        release_id: release.id.clone(),
        file_name: filename,
        package_name,
        sha256,
        size_bytes,
        min_sdk: metadata.min_sdk.map(|v| v as i64),
        target_sdk: metadata.target_sdk.map(|v| v as i64),
        version_code,
        version_name: version_name.clone(),
        supported_abis_json: supported_abis_json.clone(),
        is_universal,
        signing_fingerprint: metadata.signing_fingerprint,
        file_path_or_object_key: file_path,
        mime_type: "application/vnd.android.package-archive".to_string(),
    };

    let artifact = match artifacts::create(&state.pool, artifact_input).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "failed to create artifact record");
            let _ = state.storage.delete_artifact(&artifact_id).await;
            let _ = releases::delete(&state.pool, &release.id).await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create artifact",
            )
                .into_response();
        }
    };

    if let Err(e) = dex_metadata::store(&state, &artifact_id, profiles).await {
        tracing::error!(error = %e, "failed to store dex metadata");
        let _ = state.storage.delete_artifact(&artifact_id).await;
        let _ = releases::delete(&state.pool, &release.id).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to store dex metadata",
        )
            .into_response();
    }

    let abis: Vec<String> = serde_json::from_str(&supported_abis_json).unwrap_or_default();

    // Auto-publish if requested
    let published_at = if publish {
        if let Err(e) = releases::publish(&state.pool, &release.id).await {
            tracing::warn!(error = %e, "failed to auto-publish release");
        }
        // Re-fetch to get the actual published_at timestamp set by the DB
        releases::get_by_id(&state.pool, &release.id)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.published_at)
    } else {
        release.published_at.clone()
    };

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
                published_at,
                created_at: release.created_at,
            },
            artifact: ArtifactResponse {
                id: artifact.id,
                release_id: artifact.release_id,
                file_name: artifact.file_name,
                package_name: artifact.package_name,
                sha256: artifact.sha256,
                size_bytes: artifact.size_bytes,
                version_code: artifact.version_code,
                version_name: artifact.version_name,
                supported_abis: abis,
                is_universal: artifact.is_universal,
                created_at: artifact.created_at,
            },
        }),
    )
        .into_response()
}
