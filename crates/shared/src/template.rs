/// Substitute Scoop-style `$version` template variables in a string.
///
/// Supported variables:
/// - `$version` → full version (e.g., "3.1.2")
/// - `$majorVersion` → major component (e.g., "3")
/// - `$minorVersion` → minor component (e.g., "1")
/// - `$patchVersion` → patch component (e.g., "2")
/// - `$cleanVersion` → digits only, no separators (e.g., "312")
/// - `$underscoreVersion` → underscore-separated (e.g., "3_1_2")
/// - `$dashVersion` → dash-separated (e.g., "3-1-2")
/// - `$preReleaseVersion` → pre-release suffix (e.g., "rc1")
/// - `$buildVersion` → build metadata (e.g., "build.456")
pub fn substitute(template: &str, version: &str) -> String {
    let parts = parse_version_parts(version);

    template
        .replace("$cleanVersion", &parts.clean)
        .replace("$underscoreVersion", &parts.underscore)
        .replace("$dashVersion", &parts.dash)
        .replace("$majorVersion", &parts.major)
        .replace("$minorVersion", &parts.minor)
        .replace("$patchVersion", &parts.patch)
        .replace("$preReleaseVersion", &parts.pre_release)
        .replace("$buildVersion", &parts.build)
        .replace("$version", version)
}

struct VersionParts {
    major: String,
    minor: String,
    patch: String,
    clean: String,
    underscore: String,
    dash: String,
    pre_release: String,
    build: String,
}

fn parse_version_parts(version: &str) -> VersionParts {
    // Strip leading 'v' if present
    let v = version.strip_prefix('v').unwrap_or(version);

    // Split off build metadata (+...)
    let (base_with_pre, build) = match v.split_once('+') {
        Some((base, b)) => (base, b.to_string()),
        None => (v, String::new()),
    };

    // Split off pre-release (-...)
    let (base, pre_release) = match base_with_pre.split_once('-') {
        Some((b, pre)) => (b, pre.to_string()),
        None => (base_with_pre, String::new()),
    };

    // Split numeric components
    let components: Vec<&str> = base.split('.').collect();
    let major = components.first().unwrap_or(&"").to_string();
    let minor = components.get(1).unwrap_or(&"").to_string();
    let patch = components.get(2).unwrap_or(&"").to_string();

    // Clean: all digits concatenated (no separators)
    let clean: String = components.join("");
    let underscore = components.join("_");
    let dash = components.join("-");

    VersionParts {
        major,
        minor,
        patch,
        clean,
        underscore,
        dash,
        pre_release,
        build,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_semver() {
        let result = substitute(
            "https://example.com/$version/app-$cleanVersion.exe",
            "3.1.2",
        );
        assert_eq!(result, "https://example.com/3.1.2/app-312.exe");
    }

    #[test]
    fn all_variables() {
        let v = "3.1.2-rc1+build.456";
        assert_eq!(substitute("$version", v), "3.1.2-rc1+build.456");
        assert_eq!(substitute("$majorVersion", v), "3");
        assert_eq!(substitute("$minorVersion", v), "1");
        assert_eq!(substitute("$patchVersion", v), "2");
        assert_eq!(substitute("$cleanVersion", v), "312");
        assert_eq!(substitute("$underscoreVersion", v), "3_1_2");
        assert_eq!(substitute("$dashVersion", v), "3-1-2");
        assert_eq!(substitute("$preReleaseVersion", v), "rc1");
        assert_eq!(substitute("$buildVersion", v), "build.456");
    }

    #[test]
    fn missing_components() {
        assert_eq!(substitute("$minorVersion", "3"), "");
        assert_eq!(substitute("$patchVersion", "3.1"), "");
        assert_eq!(substitute("$preReleaseVersion", "3.1.2"), "");
    }

    #[test]
    fn leading_v_prefix() {
        assert_eq!(substitute("$majorVersion", "v3.1.2"), "3");
    }
}
