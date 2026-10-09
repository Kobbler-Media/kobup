//! Baseline profiles uploaded beside an APK as `dm_<minApi>` multipart fields. A
//! sideloading installer stages the matching one with the APK so ART compiles the app at
//! install time, the way Play does, instead of leaving it on the JIT until idle dexopt.

use crate::{AppState, apk, repositories::dex_metadata};

pub fn field_min_api(field_name: &str) -> Option<i64> {
    field_name
        .strip_prefix("dm_")?
        .parse()
        .ok()
        .filter(|api| *api > 0)
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub async fn store(
    state: &AppState,
    artifact_id: &str,
    profiles: Vec<(i64, Vec<u8>)>,
) -> Result<(), StoreError> {
    for (min_api, bytes) in profiles {
        let file_path = state
            .storage
            .store_artifact(artifact_id, &format!("{min_api}.dm"), &bytes)
            .await?;
        dex_metadata::create(
            &state.pool,
            &dex_metadata::DexMetadata {
                artifact_id: artifact_id.to_string(),
                min_api,
                sha256: apk::compute_sha256(&bytes),
                size_bytes: bytes.len() as i64,
                file_path,
            },
        )
        .await?;
    }
    Ok(())
}
