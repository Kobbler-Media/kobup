use std::sync::Arc;

use tracing::{info, warn};

use crate::DbPool;
use crate::config::Config;
use crate::repositories::{artifacts, channels, projects, releases};
use crate::storage::LocalStorage;

/// Delete releases (and their artifacts from disk) beyond the retention limit
/// for every project+channel pair. Each project may override the global default
/// via `projects.max_releases_per_channel`; a NULL override falls back to
/// `default_max_per_channel`.
pub async fn prune_old_releases(
    pool: &DbPool,
    storage: &LocalStorage,
    default_max_per_channel: usize,
) {
    let all_projects = match projects::list(pool).await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "retention: failed to list projects");
            return;
        }
    };

    for project in &all_projects {
        let project_channels = match channels::list_by_project(pool, &project.id).await {
            Ok(c) => c,
            Err(e) => {
                warn!(project_id = %project.id, error = %e, "retention: failed to list channels");
                continue;
            }
        };

        let keep = project
            .max_releases_per_channel
            .filter(|n| *n >= 0)
            .unwrap_or(default_max_per_channel as i64);

        for channel in &project_channels {
            let stale = match releases::list_for_pruning(pool, &project.id, &channel.id, keep).await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(
                        project_id = %project.id,
                        channel_id = %channel.id,
                        error = %e,
                        "retention: failed to query stale releases"
                    );
                    continue;
                }
            };

            for release in &stale {
                // Collect artifacts for disk cleanup — deleted by their actual recorded
                // file path, not a path rebuilt from the row ID (existing rows predate
                // the fix that made those match).
                let release_artifacts = artifacts::list_by_release(pool, &release.id)
                    .await
                    .unwrap_or_default();

                for artifact in &release_artifacts {
                    if let Err(e) = storage
                        .delete_artifact_file(&artifact.file_path_or_object_key)
                        .await
                    {
                        warn!(artifact_id = %artifact.id, error = %e, "retention: failed to delete artifact files");
                    }
                }

                // Delete the release (cascades to artifact rows via FK)
                if let Err(e) = releases::delete(pool, &release.id).await {
                    warn!(release_id = %release.id, error = %e, "retention: failed to delete release");
                } else {
                    info!(
                        project_slug = %project.slug,
                        channel = %channel.name,
                        release_id = %release.id,
                        version_name = %release.version_name,
                        "retention: pruned old release"
                    );
                }
            }
        }
    }
}

pub fn start_retention_loop(pool: DbPool, storage: Arc<LocalStorage>, config: Arc<Config>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            prune_old_releases(&pool, &storage, config.max_releases_per_channel).await;
        }
    });
}
