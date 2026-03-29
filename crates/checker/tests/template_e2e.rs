use astro_up_shared::template;
use astro_up_shared::version_file::VersionEntry;

/// End-to-end test: manifest with $version in autoupdate URL,
/// checker discovers version, template variables resolve in the written version file URL.
#[test]
fn template_resolution_in_version_file() {
    let dir = tempfile::tempdir().unwrap();

    // Simulate: checkver.autoupdate.url has $version template
    let autoupdate_template =
        "https://f000.backblazeb2.com/file/nina-releases/NINA-$version-Setup.exe";
    let discovered_version = "3.1.2";

    // Resolve template (this is what the checker does after finding a version)
    let resolved_url = template::substitute(autoupdate_template, discovered_version);
    assert_eq!(
        resolved_url,
        "https://f000.backblazeb2.com/file/nina-releases/NINA-3.1.2-Setup.exe"
    );

    // Write version file with resolved URL
    let entry = VersionEntry {
        url: resolved_url.clone(),
        sha256: Some("abc123".into()),
        discovered_at: chrono::Utc::now(),
        release_notes_url: None,
        pre_release: false,
    };

    let path = dir.path().join("nina-app").join("3.1.2.json");
    entry.write(&path).unwrap();

    // Read back and verify URL was resolved (not template)
    let read_back = VersionEntry::read(&path).unwrap();
    assert_eq!(
        read_back.url,
        "https://f000.backblazeb2.com/file/nina-releases/NINA-3.1.2-Setup.exe"
    );
    assert!(!read_back.url.contains("$version"));
}

#[test]
fn template_resolution_with_complex_variables() {
    let template = "https://example.com/$majorVersion/$minorVersion/app-$underscoreVersion.msi";
    let url = template::substitute(template, "4.2.1");
    assert_eq!(url, "https://example.com/4/2/app-4_2_1.msi");
}

#[test]
fn template_resolution_pre_release() {
    let template = "https://example.com/v$version/app-setup.exe";
    let url = template::substitute(template, "3.2.0-rc1");
    assert_eq!(url, "https://example.com/v3.2.0-rc1/app-setup.exe");
}
