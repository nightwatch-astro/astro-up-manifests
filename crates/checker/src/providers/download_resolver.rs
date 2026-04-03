use std::collections::HashMap;

use super::sharpcap_url;

/// Resolve a download URL using a named resolver.
/// Returns None if the resolver is unknown or fails.
pub fn resolve(name: &str, version: &str, args: &HashMap<String, String>) -> Option<String> {
    match name {
        "sharpcap" => {
            let arch = args.get("arch").map(|s| s.as_str()).unwrap_or("x64");
            sharpcap_url::sharpcap_download_url(version, arch)
        }
        other => {
            tracing::warn!("unknown download resolver: {other}");
            None
        }
    }
}
