use astro_up_compiler::manifest::load_manifests;
use std::path::Path;

#[test]
fn load_sample_manifests() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = load_manifests(&dir).unwrap();

    assert!(
        result.manifests.len() >= 4,
        "expected at least 4 manifests, got {}",
        result.manifests.len()
    );
    assert!(
        result.errors.is_empty(),
        "expected no errors, got: {:?}",
        result.errors
    );
}

#[test]
fn manifest_ids_are_unique() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = load_manifests(&dir).unwrap();

    let mut ids: Vec<&str> = result.manifests.iter().map(|m| m.id.as_str()).collect();
    ids.sort();
    let len_before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), len_before, "duplicate manifest IDs found");
}

#[test]
fn default_switches_applied() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = load_manifests(&dir).unwrap();

    let nina = result.manifests.iter().find(|m| m.id == "nina-app").unwrap();
    assert!(
        !nina.install.switches.is_empty(),
        "default inno_setup switches should be applied"
    );
}

#[test]
fn driver_has_hardware_section() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = load_manifests(&dir).unwrap();

    let driver = result.manifests.iter().find(|m| m.id == "zwo-asi-driver").unwrap();
    let hw = driver.hardware.as_ref().expect("driver should have hardware section");
    assert_eq!(hw.device_class.as_deref(), Some("Camera"));
    assert!(!hw.vid_pid.is_empty());
}

#[test]
fn manual_provider_accepted() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = load_manifests(&dir).unwrap();

    let sharpcap = result.manifests.iter().find(|m| m.id == "sharpcap").unwrap();
    let checkver = sharpcap.checkver.as_ref().unwrap();
    assert_eq!(checkver.provider, "manual");
}

#[test]
fn invalid_manifest_skipped_with_error() {
    let dir = tempfile::tempdir().unwrap();
    let bad_file = dir.path().join("bad.toml");
    std::fs::write(&bad_file, "this is not valid toml [[[").unwrap();

    let good_file = dir.path().join("good.toml");
    std::fs::write(
        &good_file,
        r#"
manifest_version = 1
id = "test"
name = "Test"
category = "utility"
type = "application"
slug = "test"
[install]
method = "zip_wrap"
"#,
    )
    .unwrap();

    let result = load_manifests(dir.path()).unwrap();

    assert_eq!(result.manifests.len(), 1);
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("bad.toml"));
}

#[test]
fn nonexistent_directory_errors() {
    let result = load_manifests(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}
