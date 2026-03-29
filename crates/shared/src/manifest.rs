use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete manifest file representing one software package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub manifest_version: u32,
    pub name: String,
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
    pub file_version: Option<bool>,
    #[serde(default)]
    pub fallback_path: Option<String>,
    #[serde(default)]
    pub fallback_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Install {
    pub method: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub elevation: bool,
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
    #[serde(default)]
    pub hash: Option<HashConfig>,
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
