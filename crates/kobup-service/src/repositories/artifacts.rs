use serde::{Deserialize, Serialize};

use crate::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Artifact {
    pub id: String,
    pub release_id: String,
    pub file_name: String,
    pub package_name: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub min_sdk: Option<i64>,
    pub target_sdk: Option<i64>,
    pub version_code: i64,
    pub version_name: String,
    pub supported_abis_json: String,
    pub is_universal: bool,
    pub signing_fingerprint: Option<String>,
    pub file_path_or_object_key: String,
    pub mime_type: String,
    pub created_at: String,
}

pub struct CreateArtifact {
    pub id: String,
    pub release_id: String,
    pub file_name: String,
    pub package_name: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub min_sdk: Option<i64>,
    pub target_sdk: Option<i64>,
    pub version_code: i64,
    pub version_name: String,
    pub supported_abis_json: String,
    pub is_universal: bool,
    pub signing_fingerprint: Option<String>,
    pub file_path_or_object_key: String,
    pub mime_type: String,
}

pub async fn list_by_release(pool: &DbPool, release_id: &str) -> super::Result<Vec<Artifact>> {
    let rows = sqlx::query_as!(
        Artifact,
        r#"SELECT id, release_id, file_name, package_name, sha256, size_bytes, min_sdk, target_sdk, version_code, version_name, supported_abis_json, is_universal as "is_universal: bool", signing_fingerprint, file_path_or_object_key, mime_type, created_at FROM artifacts WHERE release_id = ? ORDER BY created_at DESC"#,
        release_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_by_id(pool: &DbPool, id: &str) -> super::Result<Option<Artifact>> {
    let row = sqlx::query_as!(
        Artifact,
        r#"SELECT id, release_id, file_name, package_name, sha256, size_bytes, min_sdk, target_sdk, version_code, version_name, supported_abis_json, is_universal as "is_universal: bool", signing_fingerprint, file_path_or_object_key, mime_type, created_at FROM artifacts WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn create(pool: &DbPool, input: CreateArtifact) -> super::Result<Artifact> {
    let id = input.id;
    sqlx::query!(
        "INSERT INTO artifacts (id, release_id, file_name, package_name, sha256, size_bytes, min_sdk, target_sdk, version_code, version_name, supported_abis_json, is_universal, signing_fingerprint, file_path_or_object_key, mime_type) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        id,
        input.release_id,
        input.file_name,
        input.package_name,
        input.sha256,
        input.size_bytes,
        input.min_sdk,
        input.target_sdk,
        input.version_code,
        input.version_name,
        input.supported_abis_json,
        input.is_universal,
        input.signing_fingerprint,
        input.file_path_or_object_key,
        input.mime_type,
    )
    .execute(pool)
    .await?;

    Ok(Artifact {
        id,
        release_id: input.release_id,
        file_name: input.file_name,
        package_name: input.package_name,
        sha256: input.sha256,
        size_bytes: input.size_bytes,
        min_sdk: input.min_sdk,
        target_sdk: input.target_sdk,
        version_code: input.version_code,
        version_name: input.version_name,
        supported_abis_json: input.supported_abis_json,
        is_universal: input.is_universal,
        signing_fingerprint: input.signing_fingerprint,
        file_path_or_object_key: input.file_path_or_object_key,
        mime_type: input.mime_type,
        created_at: String::new(),
    })
}

pub async fn delete(pool: &DbPool, id: &str) -> super::Result<bool> {
    let result = sqlx::query!("DELETE FROM artifacts WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_by_release(pool: &DbPool, release_id: &str) -> super::Result<u64> {
    let result = sqlx::query!("DELETE FROM artifacts WHERE release_id = ?", release_id,)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn get_best_for_release(
    pool: &DbPool,
    release_id: &str,
    sdk_int: Option<i64>,
    abis: &[String],
) -> super::Result<Option<Artifact>> {
    let all = sqlx::query_as!(
        Artifact,
        r#"SELECT id, release_id, file_name, package_name, sha256, size_bytes, min_sdk, target_sdk, version_code, version_name, supported_abis_json, is_universal as "is_universal: bool", signing_fingerprint, file_path_or_object_key, mime_type, created_at FROM artifacts WHERE release_id = ?"#,
        release_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(pick_best(all, sdk_int, abis))
}

/// `abis` is the device's `Build.SUPPORTED_ABIS`, most preferred first: an arm64 phone
/// lists armeabi-v7a too, and must still get the arm64 split. Among artifacts that serve
/// the same ABI the smaller wins, so a split beats the universal APK. Without the device's
/// ABIs only a universal APK is safe to hand out.
fn pick_best(all: Vec<Artifact>, sdk_int: Option<i64>, abis: &[String]) -> Option<Artifact> {
    let mut runnable: Vec<Artifact> = all
        .into_iter()
        .filter(|a| match (a.min_sdk, sdk_int) {
            (Some(min), Some(sdk)) => min <= sdk,
            _ => true,
        })
        .collect();

    // A lone artifact is served even when not flagged universal: single-APK releases
    // uploaded before splits existed were stored that way whenever they had native libs.
    if abis.is_empty() {
        return match runnable.len() {
            1 => runnable.pop(),
            _ => runnable.into_iter().find(|a| a.is_universal),
        };
    }

    runnable
        .into_iter()
        .filter_map(|a| {
            let artifact_abis: Vec<String> =
                serde_json::from_str(&a.supported_abis_json).unwrap_or_default();
            let rank = if artifact_abis.is_empty() {
                Some(0)
            } else {
                abis.iter().position(|abi| artifact_abis.contains(abi))
            };
            rank.map(|rank| (rank, a))
        })
        .min_by_key(|(rank, a)| (*rank, a.size_bytes))
        .map(|(_, a)| a)
}
