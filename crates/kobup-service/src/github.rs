use regex::Regex;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::repositories::{artifacts, channels, projects, releases};
use crate::storage::LocalStorage;
use crate::{DbPool, apk, http};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceConfig {
    owner: String,
    repo: String,
    release_asset_pattern: Option<String>,
    #[allow(dead_code)]
    auth_mode: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn poll_project(
    pool: &DbPool,
    storage: &LocalStorage,
    config: &Config,
    project: &projects::Project,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let source_json = project
        .source_config_json
        .as_deref()
        .ok_or("missing source_config_json")?;
    let source: SourceConfig = serde_json::from_str(source_json)?;

    let client = http::client();
    let mut req = client.get(format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        source.owner, source.repo
    ));

    if let Some(ref token) = config.github_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "GitHub API error for {}/{} ({status}): {body}",
            source.owner, source.repo
        )
        .into());
    }

    let gh_release: GithubRelease = resp.json().await?;

    let pattern = source
        .release_asset_pattern
        .as_deref()
        .unwrap_or(r".*\.apk$");
    let re = Regex::new(pattern)?;

    let matching_assets: Vec<&GithubAsset> = gh_release
        .assets
        .iter()
        .filter(|a| re.is_match(&a.name))
        .collect();

    if matching_assets.is_empty() {
        info!(
            project_slug = %project.slug,
            tag = %gh_release.tag_name,
            "no matching APK assets in release"
        );
        return Ok(());
    }

    // Get default channel for the project
    let channel = channels::get_by_name(pool, &project.id, &project.default_channel)
        .await?
        .ok_or_else(|| {
            format!(
                "default channel '{}' not found for project '{}'",
                project.default_channel, project.slug
            )
        })?;

    for asset in matching_assets {
        // Check if this version already exists (by tag_name as version_name)
        let existing = releases::list_by_project(pool, &project.id).await?;
        let already_exists = existing.iter().any(|r| {
            r.version_name == gh_release.tag_name && r.source_ref.as_deref() == Some(&asset.name)
        });
        if already_exists {
            info!(
                project_slug = %project.slug,
                tag = %gh_release.tag_name,
                asset = %asset.name,
                "release already ingested, skipping"
            );
            continue;
        }

        // Download the asset
        info!(
            project_slug = %project.slug,
            asset = %asset.name,
            "downloading GitHub release asset"
        );

        let mut download_req = client.get(&asset.browser_download_url);
        if let Some(ref token) = config.github_token {
            download_req = download_req.header("Authorization", format!("Bearer {token}"));
        }

        let download_resp = download_req.send().await?;
        if !download_resp.status().is_success() {
            warn!(
                asset = %asset.name,
                status = %download_resp.status(),
                "failed to download asset, skipping"
            );
            continue;
        }

        let bytes = download_resp.bytes().await?.to_vec();

        // Extract APK metadata
        let metadata = match apk::extract_metadata(&bytes) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    asset = %asset.name,
                    error = %e,
                    "failed to extract APK metadata, skipping"
                );
                continue;
            }
        };

        let version_code = match metadata.version_code {
            Some(vc) => vc as i64,
            None => {
                warn!(
                    asset = %asset.name,
                    "could not extract version_code from APK, skipping"
                );
                continue;
            }
        };
        let package_name = metadata
            .package_name
            .or_else(|| {
                if !project.package_name.is_empty() {
                    Some(project.package_name.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let version_name = metadata
            .version_name
            .unwrap_or_else(|| gh_release.tag_name.clone());

        let sha256 = apk::compute_sha256(&bytes);
        let size_bytes = bytes.len() as i64;

        // Create release record
        let release = releases::create(
            pool,
            releases::CreateRelease {
                project_id: project.id.clone(),
                channel_id: channel.id.clone(),
                version_code,
                version_name: gh_release.tag_name.clone(),
                release_notes: gh_release.body.clone(),
                source_type: "github".to_string(),
                source_ref: Some(asset.name.clone()),
            },
        )
        .await?;

        // Auto-publish GitHub releases
        releases::publish(pool, &release.id).await?;

        // Store artifact file
        let artifact_id = uuid::Uuid::now_v7().to_string();
        let file_path = storage
            .store_artifact(&artifact_id, &asset.name, &bytes)
            .await?;

        let is_universal = metadata.abis.is_empty();
        let supported_abis_json =
            serde_json::to_string(&metadata.abis).unwrap_or_else(|_| "[]".into());

        artifacts::create(
            pool,
            artifacts::CreateArtifact {
                id: artifact_id,
                release_id: release.id.clone(),
                file_name: asset.name.clone(),
                package_name,
                sha256,
                size_bytes,
                min_sdk: metadata.min_sdk.map(|v| v as i64),
                target_sdk: metadata.target_sdk.map(|v| v as i64),
                version_code,
                version_name,
                supported_abis_json,
                is_universal,
                signing_fingerprint: metadata.signing_fingerprint,
                file_path_or_object_key: file_path,
                mime_type: "application/vnd.android.package-archive".to_string(),
            },
        )
        .await?;

        info!(
            project_slug = %project.slug,
            tag = %gh_release.tag_name,
            asset = %asset.name,
            release_id = %release.id,
            "ingested GitHub release asset"
        );
    }

    Ok(())
}

pub fn start_polling(pool: DbPool, storage: Arc<LocalStorage>, config: Arc<Config>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            config.github_poll_interval_secs,
        ));
        loop {
            interval.tick().await;

            let projects = match projects::list(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    error!(error = %e, "failed to list projects for GitHub polling");
                    continue;
                }
            };

            for project in &projects {
                if project.source_type != "github" {
                    continue;
                }

                if let Err(e) = poll_project(&pool, &storage, &config, project).await {
                    warn!(
                        project_slug = %project.slug,
                        error = %e,
                        "GitHub poll failed for project"
                    );
                }
            }
        }
    });
}
