use astro_up_shared::version::{ParsedVersion, parse};

#[test]
fn semver_ordering() {
    let v1 = parse("1.0.0", None).unwrap();
    let v2 = parse("1.1.0", None).unwrap();
    let v3 = parse("2.0.0", None).unwrap();
    assert!(v1 < v2);
    assert!(v2 < v3);
}

#[test]
fn semver_pre_release_before_stable() {
    let pre = parse("1.0.0-rc1", Some("semver")).unwrap();
    let stable = parse("1.0.0", Some("semver")).unwrap();
    assert!(pre < stable);
}

#[test]
fn lenient_semver_two_parts() {
    let v = parse("3.1", Some("semver")).unwrap();
    assert!(matches!(v, ParsedVersion::Semver(_)));
}

#[test]
fn lenient_semver_with_v_prefix() {
    let v = parse("v2.4.1", Some("semver")).unwrap();
    assert!(matches!(v, ParsedVersion::Semver(_)));
}

#[test]
fn date_format_parsing() {
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
    let v2 = parse("2026.01.15", Some("date")).unwrap();
    let v3 = parse("2026.03.29", Some("date")).unwrap();
    assert!(v1 < v2);
    assert!(v2 < v3);
}

#[test]
fn date_partial_year_month() {
    let v = parse("2026.03", Some("date")).unwrap();
    assert!(matches!(
        v,
        ParsedVersion::Date {
            year: 2026,
            month: 3,
            day: 1,
            ..
        }
    ));
}

#[test]
fn custom_regex_with_groups() {
    let v = parse("3.1 HF2", Some(r"(\d+)\.(\d+) HF(\d+)")).unwrap();
    assert!(matches!(v, ParsedVersion::Custom { .. }));
}

#[test]
fn custom_regex_ordering() {
    let v1 = parse("3.1 HF1", Some(r"(\d+)\.(\d+) HF(\d+)")).unwrap();
    let v2 = parse("3.1 HF2", Some(r"(\d+)\.(\d+) HF(\d+)")).unwrap();
    let v3 = parse("3.2 HF1", Some(r"(\d+)\.(\d+) HF(\d+)")).unwrap();
    assert!(v1 < v2);
    assert!(v2 < v3);
}
