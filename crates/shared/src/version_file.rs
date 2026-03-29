use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::version::sanitize_for_filename;

/// A single discovered version entry, stored as `versions/{id}/{version}.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
    pub discovered_at: DateTime<Utc>,
    #[serde(default)]
    pub release_notes_url: Option<String>,
    #[serde(default)]
    pub pre_release: bool,
}

impl VersionEntry {
    /// Build the file path for this version entry.
    pub fn file_path(versions_dir: &Path, package_id: &str, version: &str) -> PathBuf {
        versions_dir
            .join(package_id)
            .join(format!("{}.json", sanitize_for_filename(version)))
    }

    /// Read a version entry from a JSON file.
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Write this version entry to a JSON file, creating parent directories.
    pub fn write(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)
    }
}
