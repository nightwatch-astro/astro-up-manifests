use astro_up_shared::version::sanitize_for_filename;
use astro_up_shared::version_file::VersionEntry;
use chrono::Utc;
use std::path::Path;

/// Result of a version check that discovered a new version.
pub struct DiscoveredVersion {
    pub package_id: String,
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
    pub release_notes_url: Option<String>,
    pub pre_release: bool,
}

impl DiscoveredVersion {
    /// Write this discovered version as a JSON file.
    /// Returns the path written, or None if skipped due to empty URL.
    pub fn write(&self, versions_dir: &Path) -> Result<Option<std::path::PathBuf>, std::io::Error> {
        // Validate that URL is non-empty
        if self.url.is_empty() {
            tracing::warn!(
                "{}/{} has empty URL, skipping write",
                self.package_id,
                self.version
            );
            return Ok(None);
        }

        let safe_version = sanitize_for_filename(&self.version);
        let path = versions_dir
            .join(&self.package_id)
            .join(format!("{safe_version}.json"));

        let entry = VersionEntry {
            url: self.url.clone(),
            sha256: self.sha256.clone(),
            discovered_at: Utc::now(),
            release_notes_url: self.release_notes_url.clone(),
            pre_release: self.pre_release,
        };

        entry.write(&path)?;
        tracing::info!("new version: {}/{}", self.package_id, self.version);
        Ok(Some(path))
    }
}
