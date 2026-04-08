use astro_up_shared::manifest::{Checkver, Manifest};
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use futures::StreamExt;
use std::time::Duration;

use super::{CheckError, CheckOutcome, CheckResult};

/// Stealth JS to inject before page load — hides automation signals.
const STEALTH_JS: &str = r"
Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });
Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
window.chrome = { runtime: {} };
";

/// # Errors
///
/// Returns `CheckError` if the browser fails to launch, navigate, or extract a version.
pub async fn check(_manifest: &Manifest, checkver: &Checkver) -> Result<CheckOutcome, CheckError> {
    let url = checkver
        .url
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("url".into()))?;
    let regex_pat = checkver
        .regex
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("regex".into()))?;

    let page_timeout = Duration::from_secs(60);
    let extraction_timeout = Duration::from_secs(30);

    // Use a unique temp dir per instance to avoid Chromium SingletonLock conflicts
    let user_data_dir = tempfile::tempdir()
        .map_err(|e| CheckError::Browser(format!("failed to create temp dir: {e}")))?;

    // Launch browser with anti-detection flags
    let (mut browser, mut handler) = chromiumoxide::Browser::launch(
        chromiumoxide::BrowserConfig::builder()
            .request_timeout(page_timeout)
            .user_data_dir(user_data_dir.path())
            .arg("--disable-blink-features=AutomationControlled")
            .build()
            .map_err(|e| CheckError::Browser(format!("config error: {e}")))?,
    )
    .await
    .map_err(|e| CheckError::Browser(format!("launch error: {e}")))?;

    // Handler must be spawned or browser deadlocks
    let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let result = async {
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| CheckError::Browser(format!("navigation error: {e}")))?;

        // Inject stealth scripts on every new document (including navigations)
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new(STEALTH_JS))
            .await
            .map_err(|e| CheckError::Browser(format!("stealth inject error: {e}")))?;

        page.goto(url)
            .await
            .map_err(|e| CheckError::Browser(format!("navigation error: {e}")))?;

        page.wait_for_navigation()
            .await
            .map_err(|e| CheckError::Browser(format!("wait error: {e}")))?;

        // If a css_selector is provided, use it to interact with the page before scraping.
        // Supports: "js:..." for JavaScript evaluation, or a CSS selector to click.
        if let Some(selector) = &checkver.css_selector {
            if let Some(js) = selector.strip_prefix("js:") {
                page.evaluate(js)
                    .await
                    .map_err(|e| CheckError::Browser(format!("js eval error: {e}")))?;
            } else {
                let element = page.find_element(selector).await.map_err(|e| {
                    CheckError::Browser(format!("selector '{selector}' not found: {e}"))
                })?;
                element
                    .click()
                    .await
                    .map_err(|e| CheckError::Browser(format!("click error: {e}")))?;
            }
            // Wait for SPA content to render
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        // Extract page content with extraction timeout
        let content = tokio::time::timeout(extraction_timeout, page.content())
            .await
            .map_err(|_| CheckError::Browser("DOM extraction timeout (30s)".into()))?
            .map_err(|e| CheckError::Browser(format!("content error: {e}")))?;

        let re = regex::Regex::new(regex_pat)
            .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

        let caps = re.captures(&content).ok_or(CheckError::NoMatch)?;
        let version = caps.get(1).ok_or(CheckError::NoMatch)?.as_str().to_string();

        // Extract download URLs from <a href> attributes that match the regex.
        // This gives us asset URLs (like GitHub's asset_filter) for providers
        // where the download link is on the page.
        let mut assets = Vec::new();
        let document = scraper::Html::parse_document(&content);
        let a_selector = scraper::Selector::parse("a[href]")
            .map_err(|e| CheckError::Browser(format!("selector parse error: {e:?}")))?;
        let page_url = url::Url::parse(url).ok();
        for element in document.select(&a_selector) {
            if let Some(href) = element.value().attr("href") {
                if re.is_match(href) {
                    // Resolve relative/protocol-relative URLs against the page URL
                    let resolved = if href.starts_with("http://") || href.starts_with("https://") {
                        href.to_string()
                    } else if let Some(ref base) = page_url {
                        base.join(href)
                            .map_or_else(|_| href.to_string(), |u| u.to_string())
                    } else {
                        href.to_string()
                    };
                    let name = resolved.rsplit('/').next().unwrap_or(&resolved).to_string();
                    assets.push(super::ReleaseAsset {
                        name,
                        url: resolved,
                        size: 0,
                    });
                }
            }
        }

        // Use the first matching asset URL as the primary download URL
        let primary_url = assets.first().map(|a| a.url.clone());

        Ok(CheckOutcome::Found(CheckResult {
            version,
            url: primary_url,
            sha256: None,
            release_notes_url: None,
            pre_release: false,
            assets,
        }))
    }
    .await;

    // Clean up
    let _ = browser.close().await;
    handler_task.abort();

    result
}
