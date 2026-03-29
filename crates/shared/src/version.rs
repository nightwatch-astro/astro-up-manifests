use std::cmp::Ordering;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("failed to parse semver: {0}")]
    SemverParse(String),
    #[error("failed to parse date version: {0}")]
    DateParse(String),
    #[error("failed to parse version with custom regex: {0}")]
    RegexParse(String),
}

/// A parsed version with its format, supporting ordering.
#[derive(Debug, Clone)]
pub enum ParsedVersion {
    Semver(semver::Version),
    Date {
        year: u32,
        month: u32,
        day: u32,
        raw: String,
    },
    Custom {
        components: Vec<String>,
        raw: String,
    },
}

impl ParsedVersion {
    pub fn raw(&self) -> String {
        match self {
            Self::Semver(v) => v.to_string(),
            Self::Date { raw, .. } => raw.clone(),
            Self::Custom { raw, .. } => raw.clone(),
        }
    }
}

impl PartialEq for ParsedVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ParsedVersion {}

impl PartialOrd for ParsedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ParsedVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Semver(a), Self::Semver(b)) => a.cmp(b),
            (
                Self::Date {
                    year: y1,
                    month: m1,
                    day: d1,
                    ..
                },
                Self::Date {
                    year: y2,
                    month: m2,
                    day: d2,
                    ..
                },
            ) => y1.cmp(y2).then(m1.cmp(m2)).then(d1.cmp(d2)),
            (Self::Custom { components: a, .. }, Self::Custom { components: b, .. }) => {
                a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| {
                        // Try numeric comparison first
                        match (x.parse::<u64>(), y.parse::<u64>()) {
                            (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                            _ => x.cmp(y),
                        }
                    })
                    .find(|o| *o != Ordering::Equal)
                    .unwrap_or_else(|| a.len().cmp(&b.len()))
            }
            // Different types: fall back to raw string comparison
            _ => Ordering::Equal,
        }
    }
}

/// Parse a version string according to the specified format.
///
/// `format` values:
/// - `None` or `Some("semver")` → parse as semver (lenient)
/// - `Some("date")` → parse as date (YYYY.MM.DD or YYYY-MM-DD)
/// - Other → treat as custom regex pattern with named capture groups
pub fn parse(version: &str, format: Option<&str>) -> Result<ParsedVersion, VersionError> {
    match format.unwrap_or("semver") {
        "semver" => parse_semver(version),
        "date" => parse_date(version),
        pattern => parse_custom(version, pattern),
    }
}

fn parse_semver(version: &str) -> Result<ParsedVersion, VersionError> {
    // Strip leading 'v' prefix
    let v = version.strip_prefix('v').unwrap_or(version);

    // Try strict parse first, then lenient
    semver::Version::parse(v)
        .or_else(|_| lenient_semver::parse(v).map_err(|e| e.owned()))
        .map(ParsedVersion::Semver)
        .map_err(|e| VersionError::SemverParse(format!("{version}: {e}")))
}

fn parse_date(version: &str) -> Result<ParsedVersion, VersionError> {
    // Support YYYY.MM.DD, YYYY-MM-DD, YYYY.MM, YYYY-MM
    let parts: Vec<&str> = version.split(['.', '-']).collect();
    let year = parts
        .first()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| VersionError::DateParse(version.into()))?;
    let month = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let day = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

    Ok(ParsedVersion::Date {
        year,
        month,
        day,
        raw: version.into(),
    })
}

fn parse_custom(version: &str, pattern: &str) -> Result<ParsedVersion, VersionError> {
    let re = regex::Regex::new(pattern)
        .map_err(|e| VersionError::RegexParse(format!("{pattern}: {e}")))?;

    let caps = re
        .captures(version)
        .ok_or_else(|| VersionError::RegexParse(format!("{version} doesn't match {pattern}")))?;

    let components: Vec<String> = caps
        .iter()
        .skip(1) // skip full match
        .filter_map(|m| m.map(|m| m.as_str().to_string()))
        .collect();

    Ok(ParsedVersion::Custom {
        components,
        raw: version.into(),
    })
}

/// Sanitize a version string for use as a filename.
pub fn sanitize_for_filename(version: &str) -> String {
    version
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_strict() {
        let v = parse("3.1.2", None).unwrap();
        assert!(matches!(v, ParsedVersion::Semver(_)));
    }

    #[test]
    fn semver_lenient() {
        let v = parse("3.1", None).unwrap();
        assert!(matches!(v, ParsedVersion::Semver(_)));
    }

    #[test]
    fn semver_with_prefix() {
        let v = parse("v3.1.2", Some("semver")).unwrap();
        assert!(matches!(v, ParsedVersion::Semver(_)));
    }

    #[test]
    fn semver_ordering() {
        let v1 = parse("1.0.0", None).unwrap();
        let v2 = parse("2.0.0", None).unwrap();
        let v3 = parse("1.1.0", None).unwrap();
        assert!(v1 < v2);
        assert!(v1 < v3);
        assert!(v3 < v2);
    }

    #[test]
    fn date_format() {
        let v = parse("2026.03.29", Some("date")).unwrap();
        assert!(matches!(
            v,
            ParsedVersion::Date {
                year: 2026,
                month: 3,
                day: 29,
                ..
            }
        ));
    }

    #[test]
    fn date_ordering() {
        let v1 = parse("2025.12.01", Some("date")).unwrap();
        let v2 = parse("2026.01.01", Some("date")).unwrap();
        assert!(v1 < v2);
    }

    #[test]
    fn custom_regex() {
        let v = parse("3.1 HF2", Some(r"(\d+)\.(\d+) HF(\d+)")).unwrap();
        assert!(matches!(v, ParsedVersion::Custom { .. }));
    }

    #[test]
    fn sanitize_filename() {
        assert_eq!(sanitize_for_filename("3.1.2-rc1"), "3.1.2-rc1");
        assert_eq!(sanitize_for_filename("3.1.2+build/123"), "3.1.2+build_123");
    }
}
