use astro_up_shared::template::substitute;

#[test]
fn full_version_url_template() {
    let template = "https://github.com/daleghent/nina/releases/download/v$version/NINA-$version-Setup.exe";
    let result = substitute(template, "3.1.2");
    assert_eq!(
        result,
        "https://github.com/daleghent/nina/releases/download/v3.1.2/NINA-3.1.2-Setup.exe"
    );
}

#[test]
fn hash_url_template() {
    let template = "https://example.com/releases/$version/checksums-$cleanVersion.txt";
    let result = substitute(template, "3.1.2");
    assert_eq!(
        result,
        "https://example.com/releases/3.1.2/checksums-312.txt"
    );
}

#[test]
fn underscore_and_dash_templates() {
    let template = "app_$underscoreVersion.zip and app-$dashVersion.tar.gz";
    let result = substitute(template, "3.1.2");
    assert_eq!(result, "app_3_1_2.zip and app-3-1-2.tar.gz");
}

#[test]
fn pre_release_version() {
    let template = "v$version (pre: $preReleaseVersion, build: $buildVersion)";
    let result = substitute(template, "3.2.0-rc1+build.456");
    assert_eq!(result, "v3.2.0-rc1+build.456 (pre: rc1, build: build.456)");
}

#[test]
fn version_with_only_major() {
    let template = "$majorVersion.$minorVersion.$patchVersion";
    let result = substitute(template, "3");
    assert_eq!(result, "3..");
}

#[test]
fn version_with_two_parts() {
    let template = "$majorVersion-$minorVersion-$patchVersion";
    let result = substitute(template, "3.1");
    assert_eq!(result, "3-1-");
}

#[test]
fn no_variables_passthrough() {
    let template = "https://example.com/download/latest.exe";
    let result = substitute(template, "1.0.0");
    assert_eq!(result, "https://example.com/download/latest.exe");
}
