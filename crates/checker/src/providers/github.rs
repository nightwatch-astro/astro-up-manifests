// Copyright (C) 2024-2026 Sjors Robroek
// SPDX-License-Identifier: AGPL-3.0-only

use crate::retry_client::RetryClient;
use astro_up_shared::manifest::{Checkver, Manifest};
use regex::Regex;
use serde::Deserialize;

use super::{CheckError, CheckOutcome, CheckResult, ReleaseAsset};

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    html_url: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    browser_download_url: String,
    name: String,
    size: u64,
}

/// # Errors
///
/// Returns `CheckError` if the GitHub API request fails or no release is found.
pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &RetryClient,
) -> Result<CheckOutcome, CheckError> {
    let owner = checkver
        .owner
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("owner".into()))?;
    let repo = checkver
        .repo
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("repo".into()))?;

    let url = if checkver.include_pre_release {
        format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=1")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
    };

    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req.send().await?;

    super::check_rate_limit(&resp)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(CheckError::Other(format!(
            "GitHub API returned {status} for {owner}/{repo}"
        )));
    }

    let release = if checkver.include_pre_release {
        let releases: Vec<Release> = resp.json().await?;
        releases.into_iter().next().ok_or(CheckError::NoMatch)?
    } else {
        resp.json::<Release>().await?
    };

    // Strip tag prefix (default "v")
    let tag_prefix = checkver.tag_prefix.as_deref().unwrap_or("v");
    let version = release
        .tag_name
        .strip_prefix(tag_prefix)
        .unwrap_or(&release.tag_name)
        .to_string();

    // Build asset_filter regex from autoupdate config
    let filter_re = checkver
        .autoupdate
        .as_ref()
        .and_then(|au| au.asset_filter.as_deref())
        .and_then(|pattern| Regex::new(pattern).ok());

    // Filter and collect assets
    let filtered_assets: Vec<ReleaseAsset> = release
        .assets
        .iter()
        .filter(|a| match &filter_re {
            Some(re) => re.is_match(&a.name),
            None => true, // no filter = keep all
        })
        .map(|a| ReleaseAsset {
            name: a.name.clone(),
            url: a.browser_download_url.clone(),
            size: a.size,
        })
        .collect();

    // Primary download URL: first filtered asset, or first asset, or None
    let download_url = filtered_assets.first().map(|a| a.url.clone()).or_else(|| {
        release
            .assets
            .first()
            .map(|a| a.browser_download_url.clone())
    });

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: download_url,
        sha256: None,
        release_notes_url: Some(release.html_url),
        pre_release: release.prerelease,
        assets: filtered_assets,
    }))
}
