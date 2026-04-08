use crate::retry_client::RetryClient;
use astro_up_shared::manifest::{Checkver, Manifest};

use super::{CheckError, CheckOutcome, CheckResult};

/// Follow a URL's redirect chain and extract a version from the final URL.
///
/// Uses reqwest's redirect policy to capture the final URL without downloading
/// the body. Useful for CDNs that embed version info in the redirect target
/// (e.g., ZWO's dl.zwoastro.com).
///
/// # Errors
///
/// Returns `CheckError` if the request fails or no version match is found.
pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &RetryClient,
) -> Result<CheckOutcome, CheckError> {
    let url = checkver
        .url
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("url".into()))?;
    let regex_pat = checkver
        .regex
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("regex".into()))?;

    // Use HEAD to follow redirects without downloading the body.
    // Fall back to GET with a range header if HEAD is blocked.
    let resp = match client.head(url).send().await {
        Ok(r) if r.status().is_success() || r.status().is_redirection() => r,
        _ => {
            // Some CDNs block HEAD — try GET with Range to avoid downloading
            client.get(url).header("Range", "bytes=0-0").send().await?
        }
    };

    super::check_rate_limit(&resp)?;

    let re = regex::Regex::new(regex_pat)
        .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

    // Store the original redirect URL (not the final signed/CDN URL which may expire).
    // The download client follows the redirect chain at download time.
    let stable_url = url.to_string();

    // Check the final URL (after all redirects) for version extraction
    let final_url = resp.url().to_string();
    if let Some(caps) = re.captures(&final_url) {
        if let Some(m) = caps.get(1) {
            return Ok(CheckOutcome::Found(CheckResult {
                version: m.as_str().to_string(),
                url: Some(stable_url),
                sha256: None,
                release_notes_url: None,
                pre_release: false,
                assets: Vec::new(),
            }));
        }
    }

    // Check Content-Disposition header for filename
    if let Some(cd) = resp.headers().get("content-disposition") {
        if let Ok(val) = cd.to_str() {
            if let Some(caps) = re.captures(val) {
                if let Some(m) = caps.get(1) {
                    return Ok(CheckOutcome::Found(CheckResult {
                        version: m.as_str().to_string(),
                        url: Some(stable_url),
                        sha256: None,
                        release_notes_url: None,
                        pre_release: false,
                        assets: Vec::new(),
                    }));
                }
            }
        }
    }

    // Check Location header (if redirect wasn't followed)
    if let Some(loc) = resp.headers().get("location") {
        if let Ok(val) = loc.to_str() {
            if let Some(caps) = re.captures(val) {
                if let Some(m) = caps.get(1) {
                    return Ok(CheckOutcome::Found(CheckResult {
                        version: m.as_str().to_string(),
                        url: Some(stable_url),
                        sha256: None,
                        release_notes_url: None,
                        pre_release: false,
                        assets: Vec::new(),
                    }));
                }
            }
        }
    }

    tracing::debug!("redirect: no match in final_url={final_url}, checked headers too");
    Err(CheckError::NoMatch)
}
