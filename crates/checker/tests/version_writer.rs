use astro_up_shared::version_file::VersionEntry;
// Import from the checker crate
use astro_up_checker::version_writer::DiscoveredVersion;

#[test]
fn write_new_version() {
    let dir = tempfile::tempdir().unwrap();

    let discovered = DiscoveredVersion {
        package_id: "nina-app".into(),
        version: "3.1.2".into(),
        url: "https://example.com/nina-3.1.2.exe".into(),
        sha256: Some("abc123def456".into()),
        release_notes_url: Some("https://example.com/release".into()),
        pre_release: false,
    };

    let result = discovered.write(dir.path()).unwrap();
    assert!(result.is_some());

    let path = result.unwrap();
    assert!(path.exists());
    assert_eq!(path, dir.path().join("nina-app/3.1.2.json"));

    // Verify contents
    let entry = VersionEntry::read(&path).unwrap();
    assert_eq!(entry.url, "https://example.com/nina-3.1.2.exe");
    assert_eq!(entry.sha256.unwrap(), "abc123def456");
    assert!(!entry.pre_release);
}

#[test]
fn skip_existing_version() {
    let dir = tempfile::tempdir().unwrap();

    let discovered = DiscoveredVersion {
        package_id: "nina-app".into(),
        version: "3.1.2".into(),
        url: "https://example.com/nina-3.1.2.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    };

    // Write first time
    let result = discovered.write(dir.path()).unwrap();
    assert!(result.is_some());

    // Write again — should skip
    let result = discovered.write(dir.path()).unwrap();
    assert!(result.is_none());
}

#[test]
fn pre_release_flag() {
    let dir = tempfile::tempdir().unwrap();

    let discovered = DiscoveredVersion {
        package_id: "nina-app".into(),
        version: "3.2.0-rc1".into(),
        url: "https://example.com/nina-3.2.0-rc1.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: true,
    };

    let result = discovered.write(dir.path()).unwrap();
    let path = result.unwrap();

    let entry = VersionEntry::read(&path).unwrap();
    assert!(entry.pre_release);
}

#[test]
fn sanitize_unsafe_version_chars() {
    let dir = tempfile::tempdir().unwrap();

    let discovered = DiscoveredVersion {
        package_id: "test-app".into(),
        version: "1.0.0+build/123".into(),
        url: "https://example.com/test.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    };

    let result = discovered.write(dir.path()).unwrap();
    let path = result.unwrap();

    // Slash should be sanitized to underscore
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        "1.0.0+build_123.json"
    );
}

#[test]
fn date_version_format() {
    let dir = tempfile::tempdir().unwrap();

    let discovered = DiscoveredVersion {
        package_id: "tool".into(),
        version: "2026.03.29".into(),
        url: "https://example.com/tool.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    };

    let result = discovered.write(dir.path()).unwrap();
    let path = result.unwrap();
    assert_eq!(path.file_name().unwrap().to_str().unwrap(), "2026.03.29.json");
}
