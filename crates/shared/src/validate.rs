use crate::manifest::{Install, Manifest};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("{file}: unsupported manifest_version {version} (supported: [1])")]
    UnsupportedVersion { file: String, version: u32 },
    #[error("{file}: missing required field '{field}'")]
    MissingField { file: String, field: String },
    #[error("{file}: unknown install method '{method}'")]
    UnknownInstallMethod { file: String, method: String },
    #[error("{file}: unknown checkver provider '{provider}'")]
    UnknownProvider { file: String, provider: String },
    #[error("{file}: invalid URL in field '{field}': {url}")]
    InvalidUrl {
        file: String,
        field: String,
        url: String,
    },
}

const SUPPORTED_MANIFEST_VERSIONS: &[u32] = &[1];

const KNOWN_INSTALL_METHODS: &[&str] = &[
    "inno_setup",
    "msi",
    "nsis",
    "zip_wrap",
    "download_only",
    "exe",
];

const KNOWN_PROVIDERS: &[&str] = &[
    "github",
    "gitlab",
    "direct_url",
    "http_head",
    "html_scrape",
    "browser_scrape",
    "pe_download",
    "redirect",
    "manual",
];

/// Default silent install switches per installer type.
pub fn default_switches(method: &str) -> HashMap<String, String> {
    match method {
        "inno_setup" => HashMap::from([(
            "silent".into(),
            "/VERYSILENT /NORESTART /SUPPRESSMSGBOXES".into(),
        )]),
        "msi" => HashMap::from([("silent".into(), "/qn /norestart".into())]),
        "nsis" => HashMap::from([("silent".into(), "/S".into())]),
        _ => HashMap::new(),
    }
}

/// Apply default switches to an install section if none are specified.
pub fn apply_default_switches(install: &mut Install) {
    if install.switches.is_empty() {
        install.switches = default_switches(&install.method);
    }
}

/// Validate a parsed manifest. Returns a list of validation errors.
pub fn validate_manifest(manifest: &Manifest, file: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !SUPPORTED_MANIFEST_VERSIONS.contains(&manifest.manifest_version) {
        errors.push(ValidationError::UnsupportedVersion {
            file: file.into(),
            version: manifest.manifest_version,
        });
    }

    if manifest.id.is_empty() {
        errors.push(ValidationError::MissingField {
            file: file.into(),
            field: "id".into(),
        });
    }
    if manifest.name.is_empty() {
        errors.push(ValidationError::MissingField {
            file: file.into(),
            field: "name".into(),
        });
    }
    if manifest.category.is_empty() {
        errors.push(ValidationError::MissingField {
            file: file.into(),
            field: "category".into(),
        });
    }
    if manifest.package_type.is_empty() {
        errors.push(ValidationError::MissingField {
            file: file.into(),
            field: "type".into(),
        });
    }
    if manifest.slug.is_empty() {
        errors.push(ValidationError::MissingField {
            file: file.into(),
            field: "slug".into(),
        });
    }

    if !KNOWN_INSTALL_METHODS.contains(&manifest.install.method.as_str()) {
        errors.push(ValidationError::UnknownInstallMethod {
            file: file.into(),
            method: manifest.install.method.clone(),
        });
    }

    if let Some(checkver) = &manifest.checkver {
        if !KNOWN_PROVIDERS.contains(&checkver.provider.as_str()) {
            errors.push(ValidationError::UnknownProvider {
                file: file.into(),
                provider: checkver.provider.clone(),
            });
        }
        if let Some(url) = &checkver.url {
            if !is_valid_url(url) {
                errors.push(ValidationError::InvalidUrl {
                    file: file.into(),
                    field: "checkver.url".into(),
                    url: url.clone(),
                });
            }
        }
    }

    if let Some(homepage) = &manifest.homepage {
        if !is_valid_url(homepage) {
            errors.push(ValidationError::InvalidUrl {
                file: file.into(),
                field: "homepage".into(),
                url: homepage.clone(),
            });
        }
    }

    errors
}

fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
