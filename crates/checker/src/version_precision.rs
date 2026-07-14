// Copyright (C) 2024-2026 Sjors Robroek
// SPDX-License-Identifier: AGPL-3.0-only

use astro_up_shared::audit_types::{CheckStatus, PrecisionCheck};

/// Check whether the discovered version has sufficient precision for the resolved URL.
///
/// If the autoupdate template contains `$version` and the resolved URL was built
/// from it, the version must reconstruct the URL exactly. Otherwise, normalized
/// comparison applies.
#[must_use]
pub fn check_precision(
    resolved_url: &str,
    version: &str,
    template: Option<&str>,
) -> PrecisionCheck {
    let is_templated = template.is_some_and(|t| t.contains("$version"));

    if is_templated && !resolved_url.is_empty() {
        if let Some(tmpl) = template {
            let expected_url = tmpl.replace("$version", version);
            if resolved_url == expected_url {
                return PrecisionCheck {
                    status: CheckStatus::Pass,
                    url_contains_version: true,
                    url_version_segment: Some(version.to_string()),
                    discovered_version: version.to_string(),
                    comparison_mode: "exact".to_string(),
                };
            }
        }
        let url_segment = extract_version_from_url(resolved_url);
        PrecisionCheck {
            status: CheckStatus::Fail,
            url_contains_version: false,
            url_version_segment: url_segment,
            discovered_version: version.to_string(),
            comparison_mode: "exact".to_string(),
        }
    } else {
        let url_has_version = !resolved_url.is_empty() && resolved_url.contains(version);
        PrecisionCheck {
            status: CheckStatus::Pass,
            url_contains_version: url_has_version,
            url_version_segment: None,
            discovered_version: version.to_string(),
            comparison_mode: "normalized".to_string(),
        }
    }
}

/// Validate discovered version against declared `version_format`.
#[must_use]
pub fn validate_version_format(version: &str, version_format: Option<&str>) -> CheckStatus {
    match version_format {
        Some("semver") => {
            if semver::Version::parse(version).is_ok() || lenient_semver::parse(version).is_ok() {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            }
        }
        Some("date") => {
            let is_date = version
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '-');
            if is_date && version.len() >= 6 {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            }
        }
        Some(pattern) => match regex::Regex::new(pattern) {
            Ok(re) => {
                if re.is_match(version) {
                    CheckStatus::Pass
                } else {
                    CheckStatus::Fail
                }
            }
            Err(_) => CheckStatus::Skip,
        },
        None => CheckStatus::Pass,
    }
}

fn extract_version_from_url(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"\d+\.\d+(?:\.\d+)*").ok()?;
    re.find(url).map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_with_template() {
        let r = check_precision(
            "https://example.com/v3.2.0/setup.exe",
            "3.2.0",
            Some("https://example.com/v$version/setup.exe"),
        );
        assert_eq!(r.status, CheckStatus::Pass);
        assert_eq!(r.comparison_mode, "exact");
    }

    #[test]
    fn partial_version_fails_exact() {
        let r = check_precision(
            "https://example.com/NINA-3.2.0.9001.zip",
            "3.2",
            Some("https://example.com/NINA-$version.zip"),
        );
        assert_eq!(r.status, CheckStatus::Fail);
        assert_eq!(r.comparison_mode, "exact");
    }

    #[test]
    fn generic_url_uses_normalized() {
        let r = check_precision(
            "https://github.com/o/r/releases/download/latest/setup.exe",
            "3.2.0",
            None,
        );
        assert_eq!(r.status, CheckStatus::Pass);
        assert_eq!(r.comparison_mode, "normalized");
    }

    #[test]
    fn semver_format_valid() {
        assert_eq!(
            validate_version_format("3.2.0", Some("semver")),
            CheckStatus::Pass
        );
        assert_eq!(
            validate_version_format("bad", Some("semver")),
            CheckStatus::Fail
        );
    }

    #[test]
    fn date_format_valid() {
        assert_eq!(
            validate_version_format("2026.04.07", Some("date")),
            CheckStatus::Pass
        );
        assert_eq!(
            validate_version_format("abc", Some("date")),
            CheckStatus::Fail
        );
    }

    #[test]
    fn no_format_passes() {
        assert_eq!(validate_version_format("anything", None), CheckStatus::Pass);
    }
}
