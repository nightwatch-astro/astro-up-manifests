use astro_up_shared::manifest::{Checkver, Manifest};
use futures::StreamExt;
use std::time::Duration;

use super::{CheckError, CheckOutcome, CheckResult};

/// Stealth JS to inject before page load — hides automation signals.
const STEALTH_JS: &str = r#"
Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });
Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
window.chrome = { runtime: {} };
"#;

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

        // Inject stealth scripts before navigating to target
        page.evaluate(STEALTH_JS)
            .await
            .map_err(|e| CheckError::Browser(format!("stealth inject error: {e}")))?;

        page.goto(url)
            .await
            .map_err(|e| CheckError::Browser(format!("navigation error: {e}")))?;

        page.wait_for_navigation()
            .await
            .map_err(|e| CheckError::Browser(format!("wait error: {e}")))?;

        // Extract page content with extraction timeout
        let content = tokio::time::timeout(extraction_timeout, page.content())
            .await
            .map_err(|_| CheckError::Browser("DOM extraction timeout (30s)".into()))?
            .map_err(|e| CheckError::Browser(format!("content error: {e}")))?;

        let re = regex::Regex::new(regex_pat)
            .map_err(|e| CheckError::Other(format!("invalid regex: {e}")))?;

        let caps = re.captures(&content).ok_or(CheckError::NoMatch)?;
        let version = caps.get(1).ok_or(CheckError::NoMatch)?.as_str().to_string();

        Ok(CheckOutcome::Found(CheckResult {
            version,
            url: None,
            sha256: None,
            release_notes_url: None,
            pre_release: false,
        }))
    }
    .await;

    // Clean up
    let _ = browser.close().await;
    handler_task.abort();

    result
}
