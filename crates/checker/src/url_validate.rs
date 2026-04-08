use astro_up_shared::audit_types::{CheckStatus, UrlCheck, UrlFailureType};

use crate::retry_client::RetryClient;

/// Result of URL validation including downloaded bytes for file type detection.
pub struct UrlValidationResult {
    pub check: UrlCheck,
    /// First bytes of the download — reused for file type detection.
    pub downloaded_bytes: Vec<u8>,
}

/// Validate a URL is reachable. Fallback chain: HEAD -> GET+Range -> GET.
///
/// Returns both the check result and downloaded bytes for file detection.
#[tracing::instrument(skip_all, fields(url))]
pub async fn validate_url(client: &RetryClient, url: &str) -> UrlValidationResult {
    if url.is_empty() {
        return UrlValidationResult {
            check: UrlCheck {
                status: CheckStatus::Skip,
                url: None,
                http_status: None,
                failure_type: None,
                method_used: None,
            },
            downloaded_bytes: Vec::new(),
        };
    }

    // Attempt 1: HEAD
    match client.head(url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if resp.status().is_success() || resp.status().is_redirection() {
                return try_range_get(client, url).await;
            }
            if status == 405 || status == 403 {
                // Many servers reject HEAD but accept GET
                return try_range_get(client, url).await;
            }
            return UrlValidationResult {
                check: UrlCheck {
                    status: CheckStatus::Fail,
                    url: Some(url.to_string()),
                    http_status: Some(status),
                    failure_type: Some(classify_failure(status)),
                    method_used: Some("HEAD".to_string()),
                },
                downloaded_bytes: Vec::new(),
            };
        }
        Err(e) => {
            if e.is_timeout() || e.is_connect() {
                return try_range_get(client, url).await;
            }
            return UrlValidationResult {
                check: UrlCheck {
                    status: CheckStatus::Fail,
                    url: Some(url.to_string()),
                    http_status: None,
                    failure_type: Some(UrlFailureType::Transient),
                    method_used: Some("HEAD".to_string()),
                },
                downloaded_bytes: Vec::new(),
            };
        }
    }
}

async fn try_range_get(client: &RetryClient, url: &str) -> UrlValidationResult {
    match client
        .get(url)
        .header("Range", "bytes=0-262143")
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if status == 200 || status == 206 {
                let bytes = resp.bytes().await.unwrap_or_default().to_vec();
                tracing::debug!(http_status = status, bytes = bytes.len(), "Range GET ok");
                return UrlValidationResult {
                    check: UrlCheck {
                        status: CheckStatus::Pass,
                        url: Some(url.to_string()),
                        http_status: Some(status),
                        failure_type: None,
                        method_used: Some("GET+Range".to_string()),
                    },
                    downloaded_bytes: bytes,
                };
            }
            if status == 416 || status == 403 {
                return try_plain_get(client, url).await;
            }
            UrlValidationResult {
                check: UrlCheck {
                    status: CheckStatus::Fail,
                    url: Some(url.to_string()),
                    http_status: Some(status),
                    failure_type: Some(classify_failure(status)),
                    method_used: Some("GET+Range".to_string()),
                },
                downloaded_bytes: Vec::new(),
            }
        }
        Err(_) => try_plain_get(client, url).await,
    }
}

async fn try_plain_get(client: &RetryClient, url: &str) -> UrlValidationResult {
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if resp.status().is_success() {
                let bytes = resp.bytes().await.unwrap_or_default();
                let truncated: Vec<u8> = bytes.iter().take(262_144).copied().collect();
                UrlValidationResult {
                    check: UrlCheck {
                        status: CheckStatus::Pass,
                        url: Some(url.to_string()),
                        http_status: Some(status),
                        failure_type: None,
                        method_used: Some("GET".to_string()),
                    },
                    downloaded_bytes: truncated,
                }
            } else {
                UrlValidationResult {
                    check: UrlCheck {
                        status: CheckStatus::Fail,
                        url: Some(url.to_string()),
                        http_status: Some(status),
                        failure_type: Some(classify_failure(status)),
                        method_used: Some("GET".to_string()),
                    },
                    downloaded_bytes: Vec::new(),
                }
            }
        }
        Err(_) => UrlValidationResult {
            check: UrlCheck {
                status: CheckStatus::Fail,
                url: Some(url.to_string()),
                http_status: None,
                failure_type: Some(UrlFailureType::Transient),
                method_used: Some("GET".to_string()),
            },
            downloaded_bytes: Vec::new(),
        },
    }
}

const fn classify_failure(status: u16) -> UrlFailureType {
    match status {
        404 | 410 => UrlFailureType::Permanent,
        403 | 401 | 416 => UrlFailureType::Blocked,
        _ if status >= 500 => UrlFailureType::Transient,
        _ => UrlFailureType::Permanent,
    }
}
