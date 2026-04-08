use std::path::Path;

use astro_up_shared::audit_types::{
    AuditReport, CheckStatus, InstallMethodCheck, MatchResult, PackageValidationResult,
    ResultStatus, UrlCheck, ValidationSummary, VersionCheck,
};
use astro_up_shared::file_type;
use astro_up_shared::manifest::Manifest;
use astro_up_shared::template;

use crate::providers::{self, CheckOutcome};
use crate::retry_client::RetryClient;
use crate::url_validate;
use crate::version_precision;

/// Run a full audit of all manifests.
#[tracing::instrument(skip_all, fields(manifest_count = manifests.len()))]
pub async fn run_audit(
    manifests: &[&Manifest],
    client: &RetryClient,
    versions_dir: &Path,
    skip_url_validation: bool,
) -> AuditReport {
    let mut results = Vec::with_capacity(manifests.len());
    let mut summary = ValidationSummary::default();

    for manifest in manifests {
        let provider = manifest
            .checkver
            .as_ref()
            .map_or("none", |cv| cv.provider.as_str());
        tracing::info!(id = %manifest.id, provider, "auditing");

        let result = audit_one(manifest, client, versions_dir, skip_url_validation).await;

        match result.status {
            ResultStatus::Pass => summary.passed += 1,
            ResultStatus::Fail => {
                if result.url_reachability.status == CheckStatus::Fail {
                    summary.failed_url += 1;
                }
                if result.version_discovery.status == CheckStatus::Fail {
                    summary.failed_version += 1;
                }
                if result.install_method.status == CheckStatus::Fail {
                    summary.failed_install_method += 1;
                }
                if result
                    .version_precision
                    .as_ref()
                    .is_some_and(|p| p.status == CheckStatus::Fail)
                {
                    summary.failed_precision += 1;
                }
            }
            ResultStatus::Skip => summary.skipped += 1,
        }
        summary.total_checked += 1;
        results.push(result);
    }

    AuditReport {
        generated_at: chrono::Utc::now().to_rfc3339(),
        manifests_checked: summary.total_checked,
        manifests_passed: summary.passed,
        manifests_failed: summary.total_checked - summary.passed - summary.skipped,
        manifests_skipped: summary.skipped,
        results,
        summary,
    }
}

async fn audit_one(
    manifest: &Manifest,
    client: &RetryClient,
    _versions_dir: &Path,
    skip_url_validation: bool,
) -> PackageValidationResult {
    let provider = manifest
        .checkver
        .as_ref()
        .map_or_else(|| "none".to_string(), |cv| cv.provider.clone());
    let is_manual = provider == "manual";

    // 1. Version discovery
    let (version_check, check_result) = discover_version(manifest, client).await;
    if version_check.status == CheckStatus::Fail && !is_manual {
        return fail_early(manifest, &provider, version_check);
    }

    // 2. Resolve download URL
    let resolved_url = check_result.as_ref().map_or_else(String::new, |cr| {
        resolve_download_url(manifest, &cr.version, cr)
    });

    // 3. URL validation + bytes
    let url_result = if skip_url_validation || resolved_url.is_empty() {
        url_validate::UrlValidationResult {
            check: UrlCheck {
                status: CheckStatus::Skip,
                url: if resolved_url.is_empty() {
                    None
                } else {
                    Some(resolved_url.clone())
                },
                http_status: None,
                failure_type: None,
                method_used: None,
            },
            downloaded_bytes: Vec::new(),
        }
    } else {
        url_validate::validate_url(client, &resolved_url).await
    };

    // 4. File type + install method check
    let install_method_check =
        check_install_method(manifest, &url_result.downloaded_bytes, is_manual);

    // 5. Version precision
    // When the provider returned assets (e.g., GitHub with asset_filter),
    // the URL was resolved from assets, not the template — use normalized comparison.
    let version_precision = check_result.as_ref().map(|cr| {
        let has_assets = !cr.assets.is_empty();
        let template_url = if has_assets {
            None // Asset-resolved URL — don't compare against template
        } else {
            manifest
                .checkver
                .as_ref()
                .and_then(|cv| cv.autoupdate.as_ref())
                .and_then(|au| au.url.as_ref())
                .map(String::as_str)
        };
        version_precision::check_precision(&resolved_url, &cr.version, template_url)
    });

    // 6. Version format validation
    if let Some(ref cr) = check_result {
        let vf = manifest
            .checkver
            .as_ref()
            .and_then(|cv| cv.version_format.as_deref());
        let fmt_status = version_precision::validate_version_format(&cr.version, vf);
        if fmt_status == CheckStatus::Fail {
            tracing::warn!(id = %manifest.id, version = %cr.version, "version format mismatch");
        }
    }

    // Overall status
    let has_failure = version_check.status == CheckStatus::Fail
        || url_result.check.status == CheckStatus::Fail
        || install_method_check.status == CheckStatus::Fail
        || version_precision
            .as_ref()
            .is_some_and(|p| p.status == CheckStatus::Fail);

    let status = if has_failure {
        ResultStatus::Fail
    } else if version_check.status == CheckStatus::Skip {
        ResultStatus::Skip
    } else {
        ResultStatus::Pass
    };

    PackageValidationResult {
        id: manifest.id.clone(),
        provider,
        status,
        version_discovery: version_check,
        url_reachability: url_result.check,
        install_method: install_method_check,
        version_precision,
    }
}

fn check_install_method(manifest: &Manifest, bytes: &[u8], is_manual: bool) -> InstallMethodCheck {
    let skip = InstallMethodCheck {
        status: CheckStatus::Skip,
        declared_method: manifest.install.method.clone(),
        declared_zip_wrapped: manifest.install.zip_wrapped,
        detected_file_type: None,
        detected_method: None,
        detected_zip_wrapped: false,
        match_result: MatchResult::Skipped,
    };

    if is_manual || bytes.is_empty() {
        return InstallMethodCheck {
            match_result: if bytes.is_empty() && !is_manual {
                MatchResult::DetectionFailed
            } else {
                MatchResult::Skipped
            },
            ..skip
        };
    }

    let file_type = file_type::detect_file_type(bytes);
    let is_zip = file_type.is_zip();

    let (detected_method, detected_zip) = if is_zip {
        let has_installer = file_type::zip_contains_installer(bytes);
        if has_installer {
            ("exe".to_string(), true)
        } else {
            ("download_only".to_string(), true)
        }
    } else {
        (file_type.as_install_method().to_string(), false)
    };

    let method_ok = detected_method == manifest.install.method
        || (detected_method == "exe"
            && matches!(
                manifest.install.method.as_str(),
                "inno_setup" | "nsis" | "exe"
            ));
    let zip_ok = detected_zip == manifest.install.zip_wrapped;
    let ok = method_ok && zip_ok;

    if ok {
        tracing::debug!(id = %manifest.id, "install method matches");
    } else {
        tracing::warn!(
            id = %manifest.id,
            declared = %manifest.install.method,
            declared_zip = manifest.install.zip_wrapped,
            detected = %detected_method,
            detected_zip = detected_zip,
            "install method MISMATCH"
        );
    }

    InstallMethodCheck {
        status: if ok {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        declared_method: manifest.install.method.clone(),
        declared_zip_wrapped: manifest.install.zip_wrapped,
        detected_file_type: Some(file_type),
        detected_method: Some(detected_method),
        detected_zip_wrapped: detected_zip,
        match_result: if ok {
            MatchResult::Match
        } else {
            MatchResult::Mismatch
        },
    }
}

fn fail_early(
    manifest: &Manifest,
    provider: &str,
    version_check: VersionCheck,
) -> PackageValidationResult {
    PackageValidationResult {
        id: manifest.id.clone(),
        provider: provider.to_string(),
        status: ResultStatus::Fail,
        version_discovery: version_check,
        url_reachability: UrlCheck {
            status: CheckStatus::Skip,
            url: None,
            http_status: None,
            failure_type: None,
            method_used: None,
        },
        install_method: InstallMethodCheck {
            status: CheckStatus::Skip,
            declared_method: manifest.install.method.clone(),
            declared_zip_wrapped: manifest.install.zip_wrapped,
            detected_file_type: None,
            detected_method: None,
            detected_zip_wrapped: false,
            match_result: MatchResult::Skipped,
        },
        version_precision: None,
    }
}

async fn discover_version(
    manifest: &Manifest,
    client: &RetryClient,
) -> (VersionCheck, Option<providers::CheckResult>) {
    match providers::check_manifest(manifest, client).await {
        Ok(CheckOutcome::Found(result)) => (
            VersionCheck {
                status: CheckStatus::Pass,
                version: Some(result.version.clone()),
                error: None,
            },
            Some(result),
        ),
        Ok(CheckOutcome::Skipped { reason }) => (
            VersionCheck {
                status: CheckStatus::Skip,
                version: None,
                error: Some(reason),
            },
            None,
        ),
        Err(e) => (
            VersionCheck {
                status: CheckStatus::Fail,
                version: None,
                error: Some(e.to_string()),
            },
            None,
        ),
    }
}

fn resolve_download_url(manifest: &Manifest, version: &str, cr: &providers::CheckResult) -> String {
    if cr.assets.is_empty() {
        manifest
            .checkver
            .as_ref()
            .and_then(|cv| cv.autoupdate.as_ref())
            .and_then(|au| {
                if let Some(resolver_name) = &au.resolver {
                    return providers::download_resolver::resolve(
                        resolver_name,
                        version,
                        &au.resolver_args,
                    );
                }
                au.url
                    .as_ref()
                    .map(|tmpl| template::substitute(tmpl, version))
            })
            .or_else(|| cr.url.clone())
            .unwrap_or_default()
    } else {
        cr.url.clone().unwrap_or_default()
    }
}

/// Print human-readable summary to stderr.
pub fn print_summary(report: &AuditReport) {
    eprintln!(
        "Audit: {} checked, {} passed, {} failed, {} skipped",
        report.manifests_checked,
        report.manifests_passed,
        report.manifests_failed,
        report.manifests_skipped
    );
    if report.manifests_failed > 0 {
        let s = &report.summary;
        eprintln!(
            "  Failures: url={}, version={}, method={}, precision={}",
            s.failed_url, s.failed_version, s.failed_install_method, s.failed_precision
        );
        for r in &report.results {
            if r.status == ResultStatus::Fail {
                let mut reasons = Vec::new();
                if r.version_discovery.status == CheckStatus::Fail {
                    reasons.push(format!(
                        "version: {}",
                        r.version_discovery.error.as_deref().unwrap_or("?")
                    ));
                }
                if r.url_reachability.status == CheckStatus::Fail {
                    reasons.push(format!(
                        "url: {} ({})",
                        r.url_reachability.url.as_deref().unwrap_or("?"),
                        r.url_reachability
                            .http_status
                            .map_or_else(|| "timeout".to_string(), |s| s.to_string())
                    ));
                }
                if r.install_method.status == CheckStatus::Fail {
                    reasons.push(format!(
                        "method: {}(zip={}) != {:?}(zip={})",
                        r.install_method.declared_method,
                        r.install_method.declared_zip_wrapped,
                        r.install_method.detected_method,
                        r.install_method.detected_zip_wrapped
                    ));
                }
                if r.version_precision
                    .as_ref()
                    .is_some_and(|p| p.status == CheckStatus::Fail)
                {
                    reasons.push("precision: version too short for URL".to_string());
                }
                eprintln!("    {} — {}", r.id, reasons.join("; "));
            }
        }
    }
}
