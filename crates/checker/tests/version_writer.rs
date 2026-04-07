#![allow(clippy::expect_used)]
use astro_up_shared::version_file::VersionEntry;
// Import from the checker crate
use astro_up_checker::version_writer::DiscoveredVersion;

#[test]
fn write_new_version() {
    let dir = tempfile::tempdir().expect("tempdir should be created");

    let discovered = DiscoveredVersion {
        package_id: "nina-app".into(),
        version: "3.1.2".into(),
        url: "https://example.com/nina-3.1.2.exe".into(),
        sha256: Some("abc123def456".into()),
        release_notes_url: Some("https://example.com/release".into()),
        pre_release: false,
    };

    let result = discovered
        .write(dir.path())
        .expect("version should be written");
    assert!(result.is_some());

    let path = result.expect("write result should contain path");
    assert!(path.exists());
    assert_eq!(path, dir.path().join("nina-app/3.1.2.json"));

    // Verify contents
    let entry = VersionEntry::read(&path).expect("version entry should be readable");
    assert_eq!(entry.url, "https://example.com/nina-3.1.2.exe");
    assert_eq!(
        entry.sha256.expect("sha256 should be present"),
        "abc123def456"
    );
    assert!(!entry.pre_release);
}

#[test]
fn overwrite_existing_version() {
    let dir = tempfile::tempdir().expect("tempdir should be created");

    let discovered = DiscoveredVersion {
        package_id: "nina-app".into(),
        version: "3.1.2".into(),
        url: "https://example.com/nina-3.1.2.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    };

    // Write first time
    let result = discovered
        .write(dir.path())
        .expect("first write should succeed");
    assert!(result.is_some());

    // Write again — should overwrite
    let result = discovered
        .write(dir.path())
        .expect("overwrite should succeed");
    assert!(result.is_some());
}

#[test]
fn pre_release_flag() {
    let dir = tempfile::tempdir().expect("tempdir should be created");

    let discovered = DiscoveredVersion {
        package_id: "nina-app".into(),
        version: "3.2.0-rc1".into(),
        url: "https://example.com/nina-3.2.0-rc1.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: true,
    };

    let result = discovered
        .write(dir.path())
        .expect("pre-release write should succeed");
    let path = result.expect("write result should contain path");

    let entry = VersionEntry::read(&path).expect("version entry should be readable");
    assert!(entry.pre_release);
}

#[test]
fn sanitize_unsafe_version_chars() {
    let dir = tempfile::tempdir().expect("tempdir should be created");

    let discovered = DiscoveredVersion {
        package_id: "test-app".into(),
        version: "1.0.0+build/123".into(),
        url: "https://example.com/test.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    };

    let result = discovered
        .write(dir.path())
        .expect("write with special chars should succeed");
    let path = result.expect("write result should contain path");

    // Slash should be sanitized to underscore
    assert_eq!(
        path.file_name()
            .expect("path should have file name")
            .to_str()
            .expect("file name should be valid UTF-8"),
        "1.0.0+build_123.json"
    );
}

#[test]
fn date_version_format() {
    let dir = tempfile::tempdir().expect("tempdir should be created");

    let discovered = DiscoveredVersion {
        package_id: "tool".into(),
        version: "2026.03.29".into(),
        url: "https://example.com/tool.exe".into(),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    };

    let result = discovered
        .write(dir.path())
        .expect("date version write should succeed");
    let path = result.expect("write result should contain path");
    assert_eq!(
        path.file_name()
            .expect("path should have file name")
            .to_str()
            .expect("file name should be valid UTF-8"),
        "2026.03.29.json"
    );
}
