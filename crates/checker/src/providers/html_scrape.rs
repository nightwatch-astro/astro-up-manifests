use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;
use scraper::{Html, Selector};

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

    let resp = client.get(url).send().await?;
    super::check_rate_limit(&resp)?;
    let body = resp.text().await?;

    let re = regex::Regex::new(regex_pat)
        .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

    // Parse HTML with scraper and extract text from the DOM, then apply regex.
    // If a css_selector is configured, narrow the search to matching elements.
    let document = Html::parse_document(&body);

    if let Some(css) = checkver.css_selector.as_deref() {
        let selector = Selector::parse(css)
            .map_err(|e| CheckError::Other(format!("invalid CSS selector: {e}")))?;

        for element in document.select(&selector) {
            let text = element.text().collect::<String>();
            if let Some(caps) = re.captures(&text) {
                if let Some(m) = caps.get(1) {
                    return Ok(CheckOutcome::Found(CheckResult {
                        version: m.as_str().to_string(),
                        url: None,
                        sha256: None,
                        release_notes_url: None,
                        pre_release: false,
                    }));
                }
            }
            // Also check element attributes (href, data-version, etc.)
            for attr_val in element.value().attrs().map(|(_, v)| v) {
                if let Some(caps) = re.captures(attr_val) {
                    if let Some(m) = caps.get(1) {
                        return Ok(CheckOutcome::Found(CheckResult {
                            version: m.as_str().to_string(),
                            url: None,
                            sha256: None,
                            release_notes_url: None,
                            pre_release: false,
                        }));
                    }
                }
            }
        }
    }

    // Fallback: apply regex to the full page body (same as direct_url behavior)
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
