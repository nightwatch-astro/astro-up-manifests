use astro_up_shared::manifest::Manifest;

const SAMPLE_MANIFEST: &str = r#"
id = "nina-app"
manifest_version = 1
name = "N.I.N.A."
description = "Nighttime Imaging 'N' Astronomy"
publisher = "NINA Contributors"
homepage = "https://nighttime-imaging.eu"
category = "capture"
type = "application"
slug = "nina-app"
tags = ["imaging", "capture", "sequencer"]
aliases = ["nina"]
license = "MPL-2.0"

[detection]
method = "registry"
registry_key = "HKCU\\Software\\NINA"
registry_value = "Version"

[install]
method = "inno_setup"
scope = "user"
elevation = false

[checkver]
provider = "html_scrape"
url = "https://nighttime-imaging.eu/download/"
regex = "NINA[\\s-]+v?(\\d+\\.\\d+\\.\\d+)"
version_format = "semver"
include_pre_release = true

[checkver.autoupdate]
url = "https://f000.backblazeb2.com/file/nina-releases/NINA-$version-Setup.exe"

[backup]
config_paths = ["$LOCALAPPDATA/NINA"]
"#;

#[test]
fn parse_full_manifest() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();

    assert_eq!(manifest.id, "nina-app");
    assert_eq!(manifest.manifest_version, 1);
    assert_eq!(manifest.name, "N.I.N.A.");
    assert_eq!(manifest.category, "capture");
    assert_eq!(manifest.package_type, "application");
    assert_eq!(manifest.slug, "nina-app");
    assert_eq!(manifest.tags, vec!["imaging", "capture", "sequencer"]);
    assert_eq!(manifest.aliases, vec!["nina"]);
}

#[test]
fn parse_detection_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
    let detection = manifest.detection.unwrap();

    assert_eq!(detection.method, "registry");
    assert_eq!(detection.registry_key.unwrap(), "HKCU\\Software\\NINA");
    assert_eq!(detection.registry_value.unwrap(), "Version");
}

#[test]
fn parse_install_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();

    assert_eq!(manifest.install.method, "inno_setup");
    assert_eq!(manifest.install.scope.unwrap(), "user");
    assert!(!manifest.install.elevation);
    assert!(manifest.install.switches.is_empty());
}

#[test]
fn parse_checkver_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
    let checkver = manifest.checkver.unwrap();

    assert_eq!(checkver.provider, "html_scrape");
    assert_eq!(checkver.url.unwrap(), "https://nighttime-imaging.eu/download/");
    assert!(checkver.regex.is_some());
    assert_eq!(checkver.version_format.unwrap(), "semver");
    assert!(checkver.include_pre_release);
    assert!(checkver.autoupdate.is_some());
}

#[test]
fn parse_backup_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
    let backup = manifest.backup.unwrap();

    assert_eq!(backup.config_paths, vec!["$LOCALAPPDATA/NINA"]);
}

#[test]
fn roundtrip_serialize() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).unwrap();
    let json = serde_json::to_string(&manifest).unwrap();
    let roundtrip: Manifest = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtrip.id, manifest.id);
    assert_eq!(roundtrip.name, manifest.name);
    assert_eq!(roundtrip.category, manifest.category);
}

const MINIMAL_MANIFEST: &str = r#"
id = "simple-tool"
manifest_version = 1
name = "Simple Tool"
category = "utility"
type = "application"
slug = "simple-tool"

[install]
method = "zip_wrap"
"#;

#[test]
fn parse_minimal_manifest() {
    let manifest: Manifest = toml::from_str(MINIMAL_MANIFEST).unwrap();

    assert_eq!(manifest.id, "simple-tool");
    assert!(manifest.detection.is_none());
    assert!(manifest.checkver.is_none());
    assert!(manifest.hardware.is_none());
    assert!(manifest.backup.is_none());
    assert!(manifest.dependencies.is_none());
}

const DRIVER_MANIFEST: &str = r#"
id = "zwo-driver"
manifest_version = 1
name = "ZWO ASI Camera Driver"
category = "driver"
type = "driver"
slug = "zwo-driver"

[install]
method = "exe"
elevation = true

[hardware]
device_class = "Camera"
inf_provider = "ZWO"
vid_pid = ["03c3:120e", "03c3:120f"]
"#;

#[test]
fn parse_driver_with_hardware() {
    let manifest: Manifest = toml::from_str(DRIVER_MANIFEST).unwrap();
    let hardware = manifest.hardware.unwrap();

    assert_eq!(hardware.device_class.unwrap(), "Camera");
    assert_eq!(hardware.inf_provider.unwrap(), "ZWO");
    assert_eq!(hardware.vid_pid, vec!["03c3:120e", "03c3:120f"]);
}
