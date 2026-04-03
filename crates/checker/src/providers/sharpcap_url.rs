use rand::Rng;
use std::fmt::Write;

/// Generate a SharpCap obfuscated download URL.
///
/// Port of the Go implementation from astro-up/astro-up's
/// `internal/scrape/sharpcap.go`. The algorithm interleaves version
/// digits into a random UUID to construct a download path.
pub fn sharpcap_download_url(version: &str, arch: &str) -> Option<String> {
    if version.len() < 10 {
        return None;
    }

    let uuid = random_uuid();
    let bits = split_uuid(&uuid);
    let v = version.as_bytes();

    let dlid = if v[9] == b'.' {
        // Long version like "4.1.14247.0"
        format!(
            "a{}{}-{}{}-{}{}-{}{}-{}{}{}{}",
            v[0] as char,
            &bits[0][2..],
            v[2] as char,
            &bits[1][1..],
            v[4] as char,
            &bits[2][1..],
            v[5] as char,
            &bits[3][1..],
            v[6] as char,
            &bits[4][1..10],
            v[8] as char,
            v[7] as char,
        )
    } else {
        // Short version like "3.1.1234.0"
        format!(
            "{}{}-{}{}-{}{}-{}{}-{}{}{}",
            v[0] as char,
            &bits[0][1..],
            v[2] as char,
            &bits[1][1..],
            v[4] as char,
            &bits[2][1..],
            v[5] as char,
            &bits[3][1..],
            v[6] as char,
            &bits[4][1..11],
            v[7] as char,
        )
    };

    let mut url = format!("https://d.sharpcap.co.uk/file/{dlid}");
    if !arch.is_empty() {
        write!(url, "_{arch}").ok();
    }
    Some(url)
}

fn random_uuid() -> String {
    let mut rng = rand::rng();
    let mut b = [0u8; 16];
    rng.fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // variant 1
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_be_bytes([b[4], b[5]]),
        u16::from_be_bytes([b[6], b[7]]),
        u16::from_be_bytes([b[8], b[9]]),
        // 6 bytes as u64 (zero-padded)
        u64::from_be_bytes([0, 0, b[10], b[11], b[12], b[13], b[14], b[15]]),
    )
}

fn split_uuid(uuid: &str) -> Vec<&str> {
    uuid.split('-').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_version_url() {
        let url = sharpcap_download_url("4.1.1234.0", "x64").unwrap();
        assert!(url.starts_with("https://d.sharpcap.co.uk/file/"));
        assert!(url.ends_with("_x64"));
        // First char of first segment should be '4' (version[0])
        let path = url.strip_prefix("https://d.sharpcap.co.uk/file/").unwrap();
        assert_eq!(path.as_bytes()[0], b'4');
    }

    #[test]
    fn long_version_url() {
        let url = sharpcap_download_url("4.1.14247.0", "x64").unwrap();
        assert!(url.starts_with("https://d.sharpcap.co.uk/file/a"));
        assert!(url.ends_with("_x64"));
    }

    #[test]
    fn version_too_short() {
        assert!(sharpcap_download_url("4.1.1", "x64").is_none());
    }
}
