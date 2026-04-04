use crate::retry_client::RetryClient;
use astro_up_shared::manifest::{Checkver, Manifest};
use scraper::{Html, Selector};

use super::{CheckError, CheckOutcome, CheckResult};

/// Try to extract the download URL from an `<a>` element's href.
/// Resolves relative URLs against the base page URL.
fn extract_href(element: &scraper::ElementRef, base_url: &str) -> Option<String> {
    let href = element.value().attr("href")?;
    if href.starts_with("http://") || href.starts_with("https://") {
        Some(href.to_string())
    } else if href.starts_with('/') {
        // Resolve relative URL against base
        url::Url::parse(base_url)
            .ok()
            .and_then(|base| base.join(href).ok())
            .map(|u| u.to_string())
    } else {
        Some(href.to_string())
    }
}

/// Search `<a>` elements for hrefs matching the regex, returning both version and URL.
fn find_version_in_links(
    document: &Html,
    re: &regex::Regex,
    selector: Option<&Selector>,
    base_url: &str,
) -> Option<CheckResult> {
    let a_selector = Selector::parse("a[href]").ok()?;

    let elements: Box<dyn Iterator<Item = scraper::ElementRef>> = if let Some(scope) = selector {
        // Search <a> tags within the scoped elements
        Box::new(document.select(scope).flat_map(|el| el.select(&a_selector)))
    } else {
        Box::new(document.select(&a_selector))
    };

    for element in elements {
        if let Some(href) = element.value().attr("href") {
            if let Some(caps) = re.captures(href) {
                if let Some(m) = caps.get(1) {
                    return Some(CheckResult {
                        version: m.as_str().to_string(),
                        url: extract_href(&element, base_url),
                        sha256: None,
                        release_notes_url: None,
                        pre_release: false,
                    });
                }
            }
        }
    }

    None
}

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

    let resp = client.get(url).send().await?;
    super::check_rate_limit(&resp)?;
    let final_url = resp.url().to_string();
    let body = resp.text().await?;

    let re = regex::Regex::new(regex_pat)
        .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

    let document = Html::parse_document(&body);

    // Build optional CSS scope selector
    let scope_selector = checkver
        .css_selector
        .as_deref()
        .map(|css| {
            Selector::parse(css)
                .map_err(|e| CheckError::Other(format!("invalid CSS selector: {e}")))
        })
        .transpose()?;

    // First: search <a href> attributes for the regex — captures both version and download URL
    if let Some(result) = find_version_in_links(&document, &re, scope_selector.as_ref(), &final_url)
    {
        return Ok(CheckOutcome::Found(result));
    }

    // Second: search element text within CSS scope
    if let Some(scope) = &scope_selector {
        for element in document.select(scope) {
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
        }
    }

    // Third: apply regex to the full page body
    let caps = re.captures(&body).ok_or(CheckError::NoMatch)?;
    let version = caps.get(1).ok_or(CheckError::NoMatch)?.as_str().to_string();

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: None,
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    }))
}
