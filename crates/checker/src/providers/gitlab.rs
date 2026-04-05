use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;

use super::{CheckError, CheckOutcome, CheckResult};

#[derive(Deserialize)]
struct Tag {
    name: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TagRelease {
    tag_name: String,
    description: Option<String>,
    assets: Option<ReleaseAssets>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ReleaseAssets {
    links: Vec<AssetLink>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AssetLink {
    url: String,
}

/// # Errors
///
/// Returns `CheckError` if the GitLab API request fails or no tag is found.
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

    // URL-encode owner/repo for GitLab API
    let project_path = format!("{owner}%2F{repo}");

    // Fetch tags (enough to find a stable one if filtering)
    let per_page = if checkver.include_pre_release { 1 } else { 20 };
    let url = format!(
        "https://gitlab.com/api/v4/projects/{project_path}/repository/tags?per_page={per_page}&order_by=version"
    );

    let resp = client.get(&url).send().await?;
    super::check_rate_limit(&resp)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(CheckError::Other(format!(
            "GitLab API returned {status} for {owner}/{repo}"
        )));
    }

    let tags: Vec<Tag> = resp.json().await?;

    // Find the first matching tag based on pre-release preference
    let (tag, pre_release) = if checkver.include_pre_release {
        let tag = tags.into_iter().next().ok_or(CheckError::NoMatch)?;
        let version = tag.name.strip_prefix('v').unwrap_or(&tag.name);
        let pre = version.contains('-');
        (tag, pre)
    } else {
        // Skip pre-release tags, find first stable
        tags.into_iter()
            .find(|t| {
                let v = t.name.strip_prefix('v').unwrap_or(&t.name);
                !v.contains('-')
            })
            .map(|t| (t, false))
            .ok_or(CheckError::NoMatch)?
    };

    let version = tag.name.strip_prefix('v').unwrap_or(&tag.name).to_string();

    let release_url = format!("https://gitlab.com/{owner}/{repo}/-/tags/{}", tag.name);

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: None,
        sha256: None,
        release_notes_url: Some(release_url),
        pre_release,
    }))
}
