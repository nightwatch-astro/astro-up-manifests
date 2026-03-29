pub mod browser_scrape;
pub mod direct_url;
pub mod github;
pub mod gitlab;
pub mod html_scrape;
pub mod http_head;
pub mod manual;
pub mod pe_download;

use astro_up_shared::manifest::Manifest;
use reqwest_middleware::ClientWithMiddleware;
use thiserror::Error;

#[derive(Debug)]
pub struct CheckResult {
    pub version: String,
    pub url: Option<String>,
    pub sha256: Option<String>,
    pub release_notes_url: Option<String>,
    pub pre_release: bool,
}

#[derive(Debug)]
pub enum CheckOutcome {
    Found(CheckResult),
    Skipped { reason: String },
}

#[derive(Debug, Error)]
pub enum CheckError {
    #[error("http error: {0}")]
    Http(#[from] reqwest_middleware::Error),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("rate limited (retry-after: {retry_after:?})")]
    RateLimited { retry_after: Option<String> },
    #[error("no version match found in response")]
    NoMatch,
    #[error("provider not configured: missing {0}")]
    MissingConfig(String),
    #[error("browser scrape error: {0}")]
    Browser(String),
    #[error("PE parse error: {0}")]
    PeParse(String),
    #[error("{0}")]
    Other(String),
}

/// Run the appropriate check for a manifest based on its checkver.provider field.
pub async fn check_manifest(
    manifest: &Manifest,
    client: &ClientWithMiddleware,
) -> Result<CheckOutcome, CheckError> {
    let checkver = match &manifest.checkver {
        Some(cv) => cv,
        None => {
            return Ok(CheckOutcome::Skipped {
                reason: "no [checkver] section".into(),
            });
        }
    };

    match checkver.provider.as_str() {
        "github" => github::check(manifest, checkver, client).await,
        "gitlab" => gitlab::check(manifest, checkver, client).await,
        "direct_url" => direct_url::check(manifest, checkver, client).await,
        "http_head" => http_head::check(manifest, checkver, client).await,
        "html_scrape" => html_scrape::check(manifest, checkver, client).await,
        "browser_scrape" => browser_scrape::check(manifest, checkver).await,
        "pe_download" => pe_download::check(manifest, checkver, client).await,
        "manual" => Ok(manual::check(manifest)),
        other => Err(CheckError::Other(format!("unknown provider: {other}"))),
    }
}
