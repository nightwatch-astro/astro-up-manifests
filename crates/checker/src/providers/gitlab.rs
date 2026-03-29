use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;

use super::{CheckError, CheckOutcome, CheckResult};

#[derive(Deserialize)]
struct Tag {
    name: String,
}

#[derive(Deserialize)]
struct TagRelease {
    tag_name: String,
    description: Option<String>,
    assets: Option<ReleaseAssets>,
}

#[derive(Deserialize)]
struct ReleaseAssets {
    links: Vec<AssetLink>,
}

#[derive(Deserialize)]
struct AssetLink {
    url: String,
}

pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &ClientWithMiddleware,
) -> Result<CheckOutcome, CheckError> {
    let owner = checkver.owner.as_deref()
        .ok_or_else(|| CheckError::MissingConfig("owner".into()))?;
    let repo = checkver.repo.as_deref()
        .ok_or_else(|| CheckError::MissingConfig("repo".into()))?;

    // URL-encode owner/repo for GitLab API
    let project_path = format!("{owner}%2F{repo}");

    // Fetch latest tag
    let url = format!(
        "https://gitlab.com/api/v4/projects/{project_path}/repository/tags?per_page=1&order_by=version"
    );

    let resp = client.get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(CheckError::Other(format!(
            "GitLab API returned {status} for {owner}/{repo}"
        )));
    }

    let tags: Vec<Tag> = resp.json().await?;
    let tag = tags.into_iter().next()
        .ok_or(CheckError::NoMatch)?;

    let version = tag.name.strip_prefix('v')
        .unwrap_or(&tag.name)
        .to_string();

    // Detect pre-release from version string
    let pre_release = version.contains('-');

    if pre_release && !checkver.include_pre_release {
        return Err(CheckError::NoMatch);
    }

    let release_url = format!(
        "https://gitlab.com/{owner}/{repo}/-/tags/{}", tag.name
    );

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: None,
        sha256: None,
        release_notes_url: Some(release_url),
        pre_release,
    }))
}
