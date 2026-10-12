use regex::Regex;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::release_ingest::{self, ApkFile, NewRelease};
use crate::repositories::{projects, releases};
use crate::storage::LocalStorage;
use crate::{DbPool, http};

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
        "{}/repos/{}/{}/releases/latest",
        config.github_api_url, source.owner, source.repo
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

    let existing = releases::list_by_project(pool, &project.id).await?;
    if existing
        .iter()
        .any(|r| r.source_type == "github" && r.version_name == gh_release.tag_name)
    {
        info!(
            project_slug = %project.slug,
            tag = %gh_release.tag_name,
            "release already ingested, skipping"
        );
        return Ok(());
    }

    let mut apks = Vec::with_capacity(matching_assets.len());
    for asset in matching_assets {
        info!(
            project_slug = %project.slug,
            asset = %asset.name,
            "downloading GitHub release asset"
        );
        let mut download_req = client.get(&asset.browser_download_url);
        if let Some(ref token) = config.github_token {
            download_req = download_req.header("Authorization", format!("Bearer {token}"));
        }
        let download_resp = download_req.send().await?.error_for_status()?;
        apks.push(ApkFile {
            file_name: asset.name.clone(),
            bytes: download_resp.bytes().await?.to_vec(),
        });
    }

    let (release, artifacts) = release_ingest::ingest(
        pool,
        storage,
        project,
        NewRelease {
            apks,
            channel_name: project.default_channel.clone(),
            release_notes: gh_release.body.clone(),
            publish: true,
            version_code: None,
            version_name: Some(gh_release.tag_name.clone()),
            package_name: None,
            profiles: Vec::new(),
            source_type: "github".into(),
            source_ref: Some(gh_release.tag_name.clone()),
        },
    )
    .await?;

    info!(
        project_slug = %project.slug,
        tag = %gh_release.tag_name,
        release_id = %release.id,
        artifacts = artifacts.len(),
        "ingested GitHub release"
    );
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
