use std::collections::BTreeSet;

use crate::repositories::{artifacts, channels, projects, releases};
use crate::storage::LocalStorage;
use crate::{DbPool, apk, dex_metadata};

pub struct ApkFile {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

pub struct NewRelease {
    pub apks: Vec<ApkFile>,
    pub channel_name: String,
    pub release_notes: Option<String>,
    pub publish: bool,
    pub version_code: Option<i64>,
    pub version_name: Option<String>,
    pub package_name: Option<String>,
    pub profiles: Vec<(i64, Vec<u8>)>,
    pub source_type: String,
    pub source_ref: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DexMetadata(#[from] dex_metadata::StoreError),
}

struct ParsedApk {
    file: ApkFile,
    metadata: apk::ApkMetadata,
}

/// Every APK in `input` lands as one artifact of a single release, so ABI splits of one
/// build share a version code and devices pick between them at `latest`.
pub async fn ingest(
    pool: &DbPool,
    storage: &LocalStorage,
    project: &projects::Project,
    input: NewRelease,
) -> Result<(releases::Release, Vec<artifacts::Artifact>), IngestError> {
    if input.apks.is_empty() {
        return Err(IngestError::Invalid("missing apk field".into()));
    }
    let parsed = input
        .apks
        .into_iter()
        .map(|file| {
            let metadata = apk::extract_metadata(&file.bytes).map_err(|e| {
                IngestError::Invalid(format!("invalid APK {}: {e}", file.file_name))
            })?;
            Ok(ParsedApk { file, metadata })
        })
        .collect::<Result<Vec<_>, IngestError>>()?;

    let package_name = match input.package_name {
        Some(p) => p,
        None => agreed(&parsed, "package name", |m| m.package_name.clone())?
            .unwrap_or_else(|| project.package_name.clone()),
    };
    let version_code = match input.version_code {
        Some(vc) => vc,
        None => agreed(&parsed, "version_code", |m| m.version_code)?
            .map(i64::from)
            .ok_or_else(|| {
                IngestError::Invalid(
                    "could not extract version_code from APK; specify version_code in the upload form".into(),
                )
            })?,
    };
    let version_name = match input.version_name {
        Some(v) => v,
        None => agreed(&parsed, "version_name", |m| m.version_name.clone())?
            .unwrap_or_else(|| "unknown".into()),
    };
    agreed(&parsed, "signing key", |m| m.signing_fingerprint.clone())?;

    if !project.package_name.is_empty()
        && !package_name.is_empty()
        && project.package_name != package_name
    {
        return Err(IngestError::Invalid(format!(
            "package name mismatch: project expects '{}', got '{package_name}'",
            project.package_name
        )));
    }

    let channel = channels::get_by_name(pool, &project.id, &input.channel_name)
        .await?
        .ok_or_else(|| {
            IngestError::Invalid(format!(
                "channel '{}' not found for this project",
                input.channel_name
            ))
        })?;

    if releases::exists_version_code(pool, &project.id, &channel.id, version_code).await? {
        return Err(IngestError::Conflict(format!(
            "version_code {version_code} already exists in channel '{}'",
            input.channel_name
        )));
    }

    if project.package_name.is_empty() && !package_name.is_empty() {
        projects::update(
            pool,
            &project.id,
            &projects::UpdateProjectInput {
                package_name: Some(package_name.clone()),
                ..Default::default()
            },
        )
        .await?;
    }

    let release = releases::create(
        pool,
        releases::CreateRelease {
            project_id: project.id.clone(),
            channel_id: channel.id.clone(),
            version_code,
            version_name: version_name.clone(),
            release_notes: input.release_notes,
            source_type: input.source_type,
            source_ref: input.source_ref,
        },
    )
    .await?;

    let all_abis: BTreeSet<&String> = parsed.iter().flat_map(|p| &p.metadata.abis).collect();
    let mut stored = Vec::with_capacity(parsed.len());
    for p in &parsed {
        let abis: BTreeSet<&String> = p.metadata.abis.iter().collect();
        let artifact = CreateArgs {
            release_id: &release.id,
            package_name: &package_name,
            version_code,
            version_name: &version_name,
            is_universal: abis.is_empty() || abis == all_abis,
            profiles: &input.profiles,
        };
        match store_artifact(pool, storage, p, artifact).await {
            Ok(a) => stored.push(a),
            Err(e) => {
                for a in &stored {
                    let _ = storage.delete_artifact(&a.id).await;
                }
                let _ = releases::delete(pool, &release.id).await;
                return Err(e);
            }
        }
    }

    if input.publish {
        releases::publish(pool, &release.id).await?;
    }
    let release = releases::get_by_id(pool, &release.id)
        .await?
        .unwrap_or(release);
    Ok((release, stored))
}

/// The one value every APK that states it agrees on; splits of one build that disagree
/// were not built together.
fn agreed<T: PartialEq + std::fmt::Debug>(
    parsed: &[ParsedApk],
    what: &str,
    field: impl Fn(&apk::ApkMetadata) -> Option<T>,
) -> Result<Option<T>, IngestError> {
    let mut found: Option<T> = None;
    for value in parsed.iter().filter_map(|p| field(&p.metadata)) {
        match &found {
            Some(prev) if *prev != value => {
                return Err(IngestError::Invalid(format!(
                    "APKs disagree on {what}: {prev:?} vs {value:?}"
                )));
            }
            Some(_) => {}
            None => found = Some(value),
        }
    }
    Ok(found)
}

struct CreateArgs<'a> {
    release_id: &'a str,
    package_name: &'a str,
    version_code: i64,
    version_name: &'a str,
    is_universal: bool,
    profiles: &'a [(i64, Vec<u8>)],
}

async fn store_artifact(
    pool: &DbPool,
    storage: &LocalStorage,
    p: &ParsedApk,
    args: CreateArgs<'_>,
) -> Result<artifacts::Artifact, IngestError> {
    let id = uuid::Uuid::now_v7().to_string();
    let file_path = storage
        .store_artifact(&id, &p.file.file_name, &p.file.bytes)
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::InvalidInput => IngestError::Invalid("invalid filename".into()),
            _ => IngestError::Io(e),
        })?;
    let result = async {
        let artifact = artifacts::create(
            pool,
            artifacts::CreateArtifact {
                id: id.clone(),
                release_id: args.release_id.to_string(),
                file_name: p.file.file_name.clone(),
                package_name: args.package_name.to_string(),
                sha256: apk::compute_sha256(&p.file.bytes),
                size_bytes: p.file.bytes.len() as i64,
                min_sdk: p.metadata.min_sdk.map(i64::from),
                target_sdk: p.metadata.target_sdk.map(i64::from),
                version_code: args.version_code,
                version_name: args.version_name.to_string(),
                supported_abis_json: serde_json::to_string(&p.metadata.abis)
                    .unwrap_or_else(|_| "[]".into()),
                is_universal: args.is_universal,
                signing_fingerprint: p.metadata.signing_fingerprint.clone(),
                file_path_or_object_key: file_path,
                mime_type: "application/vnd.android.package-archive".to_string(),
            },
        )
        .await?;
        // ABI splits share their dex, so one set of profiles fits every split.
        dex_metadata::store(pool, storage, &id, args.profiles).await?;
        Ok(artifact)
    }
    .await;
    if result.is_err() {
        let _ = storage.delete_artifact(&id).await;
    }
    result
}
