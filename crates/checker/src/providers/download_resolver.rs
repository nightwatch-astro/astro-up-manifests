use std::collections::HashMap;
use std::hash::BuildHasher;

use super::sharpcap_url;

/// Resolve a download URL using a named resolver.
/// Returns None if the resolver is unknown or fails.
pub fn resolve<S: BuildHasher>(
    name: &str,
    version: &str,
    args: &HashMap<String, String, S>,
) -> Option<String> {
    match name {
        "sharpcap" => {
            let arch = args.get("arch").map_or("x64", String::as_str);
            sharpcap_url::sharpcap_download_url(version, arch)
        }
        other => {
            tracing::warn!("unknown download resolver: {other}");
            None
        }
    }
}
