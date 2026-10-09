use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// User-level config stored at ~/.config/kobup/config.json
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub server: Option<String>,
    pub token: Option<String>,
}

/// Project-level config stored at .kobup.json in the repo root
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub project_slug: String,
    pub project_name: String,
    pub package_name: String,
    pub channel: String,
    pub server: String,
}

impl UserConfig {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kobup")
            .join("config.json")
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}

impl ProjectConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(".kobup.json")?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(".kobup.json", data)?;
        Ok(())
    }
}
