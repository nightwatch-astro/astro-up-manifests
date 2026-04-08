use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete manifest file representing one software package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub manifest_version: u32,
    pub name: String,
    /// When true, the checker and compiler skip this manifest entirely.
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub category: String,
    #[serde(rename = "type")]
    pub package_type: String,
    pub slug: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    /// Icon reference: resolves to `assets/icons/{value}.png`.
    /// Falls back to publisher-derived slug if not set.
    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub detection: Option<Detection>,
    pub install: Install,
    #[serde(default)]
    pub checkver: Option<Checkver>,
    #[serde(default)]
    pub hardware: Option<Hardware>,
    #[serde(default)]
    pub backup: Option<Backup>,
    #[serde(default)]
    pub dependencies: Option<Dependencies>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub method: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub registry_key: Option<String>,
    #[serde(default)]
    pub registry_value: Option<String>,
    #[serde(default)]
    pub version_regex: Option<String>,
    #[serde(default)]
    pub product_code: Option<String>,
    #[serde(default)]
    pub upgrade_code: Option<String>,
    #[serde(default)]
    pub inf_provider: Option<String>,
    #[serde(default)]
    pub device_class: Option<String>,
    #[serde(default)]
    pub inf_name: Option<String>,
    #[serde(default)]
    pub file_version: Option<bool>,
    #[serde(default)]
    pub fallback_path: Option<String>,
    #[serde(default)]
    pub fallback_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Install {
    /// Installer framework: `inno_setup`, `nsis`, `msi`, `exe`, `download_only`.
    pub method: String,
    /// Whether the download is a zip containing the installer.
    /// A plain zip (portable app) uses `download_only` + `zip_wrapped = true`.
    #[serde(default)]
    pub zip_wrapped: bool,
    /// Subfolder inside the ZIP to find the installer (e.g., `"x64"` for 64-bit).
    /// When set, only files under this path are considered for installation.
    #[serde(default)]
    pub zip_inner_path: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub elevation: Option<String>,
    #[serde(default)]
    pub switches: HashMap<String, String>,
    #[serde(default)]
    pub exit_codes: Vec<i32>,
    #[serde(default)]
    pub success_codes: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkver {
    pub provider: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub version_format: Option<String>,
    #[serde(default)]
    pub include_pre_release: bool,
    #[serde(default)]
    pub css_selector: Option<String>,
    /// Tag prefix for GitHub/GitLab releases. Default: "v".
    /// Set to "" for repos that tag without prefix (e.g., `NexDome` uses "4.0.0" not "v4.0.0").
    #[serde(default)]
    pub tag_prefix: Option<String>,
    #[serde(default)]
    pub hash: Option<HashConfig>,
    #[serde(default)]
    pub autoupdate: Option<Autoupdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub jsonpath: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autoupdate {
    #[serde(default)]
    pub url: Option<String>,
    /// Named download URL resolver for vendors with non-standard URL generation.
    /// The checker looks up a function by this name to generate the download URL
    /// from the discovered version string.
    #[serde(default)]
    pub resolver: Option<String>,
    /// Extra parameters passed to the resolver (e.g., architecture).
    #[serde(default)]
    pub resolver_args: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub hash: Option<HashConfig>,
    /// Regex filter for GitHub release assets. Only assets whose filename
    /// matches this pattern are stored in the catalog. Applied by the checker
    /// when `provider = "github"`. Example: `"win64\\.exe$"` keeps only
    /// Windows 64-bit EXE installers.
    #[serde(default)]
    pub asset_filter: Option<String>,
    /// Skip browser user-agent for downloads. Some CDNs (e.g., `SourceForge`)
    /// serve JS redirect pages to browsers instead of following HTTP redirects.
    /// When `true`, the checker and lifecycle script use the default UA.
    #[serde(default)]
    pub skip_browser_ua: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hardware {
    #[serde(default)]
    pub device_class: Option<String>,
    #[serde(default)]
    pub inf_provider: Option<String>,
    #[serde(default)]
    pub vid_pid: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    #[serde(default)]
    pub config_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub requires: Vec<String>,
}

impl Manifest {
    /// Whether this manifest's download should skip the browser user-agent.
    #[must_use]
    pub fn skip_browser_ua(&self) -> bool {
        self.checkver
            .as_ref()
            .and_then(|cv| cv.autoupdate.as_ref())
            .is_some_and(|au| au.skip_browser_ua)
    }
}
