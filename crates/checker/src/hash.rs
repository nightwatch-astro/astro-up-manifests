use std::fmt::Write;

use astro_up_shared::manifest::HashConfig;
use astro_up_shared::template;
use reqwest_middleware::ClientWithMiddleware;
use sha2::{Digest, Sha256};

/// Discover the SHA256 hash for a download URL using the manifest's hash config.
/// One method per manifest based on which fields are present.
pub async fn discover_hash(
    config: Option<&HashConfig>,
    download_url: &str,
    version: &str,
    client: &ClientWithMiddleware,
) -> Option<String> {
    let config = config?;

    if let Some(hash_url) = &config.url {
        if let Some(regex_pat) = &config.regex {
            return hash_from_url_regex(hash_url, regex_pat, version, client).await;
        }
    }

    if let Some(jsonpath) = &config.jsonpath {
        if let Some(hash_url) = &config.url {
            return hash_from_jsonpath(hash_url, jsonpath, version, client).await;
        }
    }

    // No hash config fields → download and compute
    hash_from_download(download_url, client).await
}

async fn hash_from_url_regex(
    url_template: &str,
    regex_pat: &str,
    version: &str,
    client: &ClientWithMiddleware,
) -> Option<String> {
    let url = template::substitute(url_template, version);
    let body = client.get(&url).send().await.ok()?.text().await.ok()?;

    let re = regex::Regex::new(regex_pat).ok()?;
    let caps = re.captures(&body)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

async fn hash_from_jsonpath(
    url_template: &str,
    jsonpath: &str,
    version: &str,
    client: &ClientWithMiddleware,
) -> Option<String> {
    let url = template::substitute(url_template, version);
    let body = client.get(&url).send().await.ok()?.text().await.ok()?;

    // Simple JSONPath: split by '.' and traverse
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    let mut current = &json;
    for key in jsonpath.trim_start_matches("$.").split('.') {
        current = current.get(key)?;
    }
    current.as_str().map(std::string::ToString::to_string)
}

async fn hash_from_download(url: &str, client: &ClientWithMiddleware) -> Option<String> {
    let bytes = client.get(url).send().await.ok()?.bytes().await.ok()?;
    let hash = Sha256::digest(&bytes);
    Some(hash.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    }))
}
