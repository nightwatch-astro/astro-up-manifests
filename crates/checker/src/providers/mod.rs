// Copyright (C) 2024-2026 Sjors Robroek
// SPDX-License-Identifier: AGPL-3.0-only

pub mod browser_scrape;
pub mod direct_url;
pub mod download_resolver;
pub mod github;
pub mod gitlab;
pub mod html_scrape;
pub mod http_head;
pub mod manual;
pub mod pe_download;
pub mod redirect;
pub mod sharpcap_url;
pub mod static_version;

use crate::retry_client::RetryClient;
use astro_up_shared::manifest::Manifest;
use thiserror::Error;

#[derive(Debug, Default)]
pub struct CheckResult {
    pub version: String,
    pub url: Option<String>,
    pub sha256: Option<String>,
    pub release_notes_url: Option<String>,
    pub pre_release: bool,
    /// All release assets matching the manifest's `asset_filter` (GitHub provider only).
    pub assets: Vec<ReleaseAsset>,
}

/// A downloadable asset from a GitHub release.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug)]
pub enum CheckOutcome {
    Found(CheckResult),
    Skipped { reason: String },
}

#[derive(Debug, Error)]
pub enum CheckError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("rate limited (retry-after: {retry_after:?})")]
    RateLimited { retry_after: Option<String> },
    #[error("no version match found in response")]
    NoMatch,
    #[error("provider not configured: missing {0}")]
    MissingConfig(String),
    #[error("browser scrape error: {0}")]
    Browser(String),
    #[error("PE parse error: {0}")]
    PeParse(String),
    #[error("{0}")]
    Other(String),
}

/// Check an HTTP response for rate limiting (429) and return a `RateLimited` error if detected.
///
/// # Errors
///
/// Returns `CheckError::RateLimited` if the response status is 429.
pub fn check_rate_limit(resp: &reqwest::Response) -> Result<(), CheckError> {
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);
        return Err(CheckError::RateLimited { retry_after });
    }
    Ok(())
}

/// Run the appropriate check for a manifest based on its checkver.provider field.
///
/// # Errors
///
/// Returns `CheckError` if the version check fails.
pub async fn check_manifest(
    manifest: &Manifest,
    client: &RetryClient,
) -> Result<CheckOutcome, CheckError> {
    let Some(checkver) = &manifest.checkver else {
        return Ok(CheckOutcome::Skipped {
            reason: "no [checkver] section".into(),
        });
    };

    match checkver.provider.as_str() {
        "github" => github::check(manifest, checkver, client).await,
        "gitlab" => gitlab::check(manifest, checkver, client).await,
        "direct_url" => direct_url::check(manifest, checkver, client).await,
        "http_head" => http_head::check(manifest, checkver, client).await,
        "html_scrape" => html_scrape::check(manifest, checkver, client).await,
        "browser_scrape" => browser_scrape::check(manifest, checkver).await,
        "pe_download" => pe_download::check(manifest, checkver, client).await,
        "redirect" => redirect::check(manifest, checkver, client).await,
        "manual" => Ok(manual::check(manifest)),
        "static" => static_version::check(manifest, checkver),
        other => Err(CheckError::Other(format!("unknown provider: {other}"))),
    }
}
