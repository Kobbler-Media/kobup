use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct LocalStorage {
    root: PathBuf,
}

/// Rejects anything but a single safe path component: no empty name, no
/// `.`/`..`, no embedded separators, no control characters. Callers pass
/// attacker-controlled filenames (multipart `file_name()`, GitHub release
/// asset names) straight through to a filesystem join, so this is the only
/// thing standing between a crafted name and writing outside the artifact
/// directory.
fn sanitize_filename(filename: &str) -> Result<&str, io::Error> {
    let name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| *n == filename);
    match name {
        Some(n) if n != "." && n != ".." && !n.chars().any(|c| c.is_control()) => Ok(n),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid filename",
        )),
    }
}

impl LocalStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn artifact_dir(&self, artifact_id: &str) -> PathBuf {
        self.root.join("artifacts").join(artifact_id)
    }

    pub async fn store_artifact(
        &self,
        artifact_id: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<String, std::io::Error> {
        let filename = sanitize_filename(filename)?;
        let dir = self.artifact_dir(artifact_id);
        fs::create_dir_all(&dir).await?;
        let path = dir.join(filename);
        fs::write(&path, data).await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn artifact_path(&self, artifact_id: &str, filename: &str) -> PathBuf {
        self.artifact_dir(artifact_id).join(filename)
    }

    pub async fn delete_artifact(&self, artifact_id: &str) -> Result<(), std::io::Error> {
        let dir = self.artifact_dir(artifact_id);
        if dir.exists() {
            fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    /// Deletes the directory containing a stored artifact file, given the
    /// exact path recorded in `artifacts.file_path_or_object_key` — rather
    /// than rebuilding the path from a DB-row ID, which existing rows can't
    /// be trusted to match (see `repositories::artifacts::create`'s history:
    /// it used to mint its own ID independent of the one already used for
    /// the storage folder).
    pub async fn delete_artifact_file(&self, file_path: &str) -> Result<(), std::io::Error> {
        if let Some(dir) = Path::new(file_path).parent()
            && dir.exists()
        {
            fs::remove_dir_all(dir).await?;
        }
        Ok(())
    }

    fn icons_dir(&self) -> PathBuf {
        self.root.join("icons")
    }

    pub fn icon_path(&self, project_id: &str) -> PathBuf {
        self.icons_dir().join(format!("{project_id}.png"))
    }

    pub async fn store_icon(
        &self,
        project_id: &str,
        data: &[u8],
    ) -> Result<String, std::io::Error> {
        let dir = self.icons_dir();
        fs::create_dir_all(&dir).await?;
        let path = self.icon_path(project_id);
        fs::write(&path, data).await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub async fn ensure_dirs(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(self.root.join("artifacts")).await?;
        fs::create_dir_all(self.root.join("icons")).await
    }
}
