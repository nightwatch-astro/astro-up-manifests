use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;

use super::{CheckError, CheckOutcome, CheckResult};

pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &ClientWithMiddleware,
) -> Result<CheckOutcome, CheckError> {
    let url = checkver.url.as_deref()
        .ok_or_else(|| CheckError::MissingConfig("url".into()))?;
    let regex_pat = checkver.regex.as_deref()
        .ok_or_else(|| CheckError::MissingConfig("regex".into()))?;

    let resp = client.head(url).send().await?;

    let re = regex::Regex::new(regex_pat)
        .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

    // Check Content-Disposition header
    if let Some(cd) = resp.headers().get("content-disposition") {
        if let Ok(val) = cd.to_str() {
            if let Some(caps) = re.captures(val) {
                if let Some(m) = caps.get(1) {
                    return Ok(CheckOutcome::Found(CheckResult {
                        version: m.as_str().to_string(),
                        url: Some(url.to_string()),
                        sha256: None,
                        release_notes_url: None,
                        pre_release: false,
                    }));
                }
            }
        }
    }

    // Check Location header (redirect URL)
    if let Some(loc) = resp.headers().get("location") {
        if let Ok(val) = loc.to_str() {
            if let Some(caps) = re.captures(val) {
                if let Some(m) = caps.get(1) {
                    return Ok(CheckOutcome::Found(CheckResult {
                        version: m.as_str().to_string(),
                        url: Some(val.to_string()),
                        sha256: None,
                        release_notes_url: None,
                        pre_release: false,
                    }));
                }
            }
        }
    }

    // Check final URL (after redirects)
    let final_url = resp.url().to_string();
    if let Some(caps) = re.captures(&final_url) {
        if let Some(m) = caps.get(1) {
            return Ok(CheckOutcome::Found(CheckResult {
                version: m.as_str().to_string(),
                url: Some(final_url),
                sha256: None,
                release_notes_url: None,
                pre_release: false,
            }));
        }
    }

    Err(CheckError::NoMatch)
}
