#![allow(clippy::expect_used)]

// Copyright (C) 2024-2026 Sjors Robroek
// SPDX-License-Identifier: AGPL-3.0-only

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
elevation = "prohibited"

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
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).expect("sample manifest should parse");

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
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).expect("sample manifest should parse");
    let detection = manifest
        .detection
        .expect("sample manifest should have detection section");

    assert_eq!(detection.method, "registry");
    assert_eq!(
        detection
            .registry_key
            .expect("detection should have registry_key"),
        "HKCU\\Software\\NINA"
    );
    assert_eq!(
        detection
            .registry_value
            .expect("detection should have registry_value"),
        "Version"
    );
}

#[test]
fn parse_install_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).expect("sample manifest should parse");

    assert_eq!(manifest.install.method, "inno_setup");
    assert_eq!(
        manifest.install.scope.expect("install should have scope"),
        "user"
    );
    assert_eq!(manifest.install.elevation, Some("prohibited".to_string()));
    assert!(manifest.install.switches.is_empty());
}

#[test]
fn parse_checkver_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).expect("sample manifest should parse");
    let checkver = manifest
        .checkver
        .expect("sample manifest should have checkver section");

    assert_eq!(checkver.provider, "html_scrape");
    assert_eq!(
        checkver.url.expect("checkver should have url"),
        "https://nighttime-imaging.eu/download/"
    );
    assert!(checkver.regex.is_some());
    assert_eq!(
        checkver
            .version_format
            .expect("checkver should have version_format"),
        "semver"
    );
    assert!(checkver.include_pre_release);
    assert!(checkver.autoupdate.is_some());
}

#[test]
fn parse_backup_section() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).expect("sample manifest should parse");
    let backup = manifest
        .backup
        .expect("sample manifest should have backup section");

    assert_eq!(backup.config_paths, vec!["$LOCALAPPDATA/NINA"]);
}

#[test]
fn roundtrip_serialize() {
    let manifest: Manifest = toml::from_str(SAMPLE_MANIFEST).expect("sample manifest should parse");
    let json = serde_json::to_string(&manifest).expect("manifest should serialize to JSON");
    let roundtrip: Manifest =
        serde_json::from_str(&json).expect("JSON should deserialize back to Manifest");

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
method = "exe"
zip_wrapped = true
"#;

#[test]
fn parse_minimal_manifest() {
    let manifest: Manifest =
        toml::from_str(MINIMAL_MANIFEST).expect("minimal manifest should parse");

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
elevation = "required"

[hardware]
device_class = "Camera"
inf_provider = "ZWO"
vid_pid = ["03c3:120e", "03c3:120f"]
"#;

#[test]
fn parse_driver_with_hardware() {
    let manifest: Manifest = toml::from_str(DRIVER_MANIFEST).expect("driver manifest should parse");
    let hardware = manifest
        .hardware
        .expect("driver manifest should have hardware section");

    assert_eq!(
        hardware
            .device_class
            .expect("hardware should have device_class"),
        "Camera"
    );
    assert_eq!(
        hardware
            .inf_provider
            .expect("hardware should have inf_provider"),
        "ZWO"
    );
    assert_eq!(hardware.vid_pid, vec!["03c3:120e", "03c3:120f"]);
}
