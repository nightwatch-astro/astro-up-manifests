use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Skip,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlFailureType {
    Permanent,
    Transient,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    PeExe,
    Zip,
    Msi,
    Nsis,
    InnoSetup,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchResult {
    Match,
    Mismatch,
    Skipped,
    DetectionFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlCheck {
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_type: Option<UrlFailureType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCheck {
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMethodCheck {
    pub status: CheckStatus,
    pub declared_method: String,
    pub declared_zip_wrapped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_file_type: Option<FileType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_method: Option<String>,
    pub detected_zip_wrapped: bool,
    pub match_result: MatchResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecisionCheck {
    pub status: CheckStatus,
    pub url_contains_version: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_version_segment: Option<String>,
    pub discovered_version: String,
    pub comparison_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageValidationResult {
    pub id: String,
    pub provider: String,
    pub status: ResultStatus,
    pub version_discovery: VersionCheck,
    pub url_reachability: UrlCheck,
    pub install_method: InstallMethodCheck,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_precision: Option<PrecisionCheck>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_checked: u32,
    pub passed: u32,
    pub failed_url: u32,
    pub failed_version: u32,
    pub failed_install_method: u32,
    pub failed_precision: u32,
    pub skipped: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub generated_at: String,
    pub manifests_checked: u32,
    pub manifests_passed: u32,
    pub manifests_failed: u32,
    pub manifests_skipped: u32,
    pub results: Vec<PackageValidationResult>,
    pub summary: ValidationSummary,
}

impl FileType {
    /// Convert detected file type to the corresponding `install.method` value.
    #[must_use]
    pub const fn as_install_method(&self) -> &'static str {
        match self {
            Self::InnoSetup => "inno_setup",
            Self::Nsis => "nsis",
            Self::Msi => "msi",
            Self::PeExe | Self::Unknown => "exe",
            Self::Zip => "download_only",
        }
    }

    /// Check if this detected file type is compatible with the declared install method.
    #[must_use]
    pub fn matches_install_method(&self, method: &str) -> bool {
        matches!(
            (self, method),
            (Self::InnoSetup, "inno_setup" | "exe")
                | (Self::Nsis, "nsis" | "exe")
                | (Self::Msi, "msi" | "wix")
                | (Self::PeExe, "exe" | "inno_setup" | "nsis" | "burn")
                | (Self::Zip, "download_only")
        )
    }

    #[must_use]
    pub const fn is_zip(&self) -> bool {
        matches!(self, Self::Zip)
    }
}
