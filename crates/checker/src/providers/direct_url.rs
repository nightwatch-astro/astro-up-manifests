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

    let body = client.get(url).send().await?.text().await?;

    let re = regex::Regex::new(regex_pat)
        .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

    let caps = re.captures(&body).ok_or(CheckError::NoMatch)?;
    let version = caps.get(1)
        .ok_or(CheckError::NoMatch)?
        .as_str()
        .to_string();

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: None,
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    }))
}
