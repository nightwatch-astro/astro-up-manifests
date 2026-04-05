use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;

use super::{CheckError, CheckOutcome, CheckResult};

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    html_url: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Asset {
    browser_download_url: String,
    name: String,
}

/// # Errors
///
/// Returns `CheckError` if the GitHub API request fails or no release is found.
pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &ClientWithMiddleware,
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
        // Fetch all releases and pick the first (latest)
        format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=1")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
    };

    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    // Authenticate if GITHUB_TOKEN is available (5000 req/hr vs 60 unauthenticated)
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

    // Strip leading 'v' from tag
    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    // Find download URL from assets (first .exe, .msi, .zip, or first asset)
    let download_url = release
        .assets
        .first()
        .map(|a| a.browser_download_url.clone());

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: download_url,
        sha256: None,
        release_notes_url: Some(release.html_url),
        pre_release: release.prerelease,
    }))
}
