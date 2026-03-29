use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Per-manifest checker state, tracking failure counts for auto-issue creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestState {
    pub consecutive_failures: u32,
    pub last_checked: DateTime<Utc>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub issue_number: Option<u64>,
}

/// The full checker state file (`checker-state.json`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckerState {
    #[serde(flatten)]
    pub manifests: HashMap<String, ManifestState>,
}

impl CheckerState {
    /// Read state from a JSON file. Returns default (empty) if file doesn't exist.
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Write state to a JSON file.
    pub fn write(&self, path: &Path) -> Result<(), std::io::Error> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)
    }

    /// Record a successful check for a manifest. Resets failure count.
    pub fn record_success(&mut self, id: &str) {
        let entry = self.manifests.entry(id.to_string()).or_insert_with(|| ManifestState {
            consecutive_failures: 0,
            last_checked: Utc::now(),
            last_error: None,
            issue_number: None,
        });
        entry.consecutive_failures = 0;
        entry.last_checked = Utc::now();
        entry.last_error = None;
    }

    /// Record a failed check for a manifest. Increments failure count.
    pub fn record_failure(&mut self, id: &str, error: &str) {
        let entry = self.manifests.entry(id.to_string()).or_insert_with(|| ManifestState {
            consecutive_failures: 0,
            last_checked: Utc::now(),
            last_error: None,
            issue_number: None,
        });
        entry.consecutive_failures += 1;
        entry.last_checked = Utc::now();
        entry.last_error = Some(error.to_string());
    }

    /// Check if a manifest has reached the persistent failure threshold (8 consecutive).
    pub fn needs_issue(&self, id: &str) -> bool {
        self.manifests.get(id).map_or(false, |s| {
            s.consecutive_failures >= 8 && s.issue_number.is_none()
        })
    }

    /// Check if a manifest's issue should be closed (was failing, now succeeded).
    pub fn should_close_issue(&self, id: &str) -> Option<u64> {
        self.manifests.get(id).and_then(|s| {
            if s.consecutive_failures == 0 {
                s.issue_number
            } else {
                None
            }
        })
    }
}
