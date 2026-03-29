use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;

use super::{CheckError, CheckOutcome, CheckResult};

pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &ClientWithMiddleware,
) -> Result<CheckOutcome, CheckError> {
    let url = checkver
        .url
        .as_deref()
        .ok_or_else(|| CheckError::MissingConfig("url".into()))?;

    // Download the executable
    let resp = client.get(url).send().await?;
    super::check_rate_limit(&resp)?;
    let bytes = resp.bytes().await?;

    // Parse PE and extract FileVersion from VS_VERSION_INFO resource
    let pe = pelite::PeFile::from_bytes(&bytes).map_err(|e| CheckError::PeParse(format!("{e}")))?;

    let resources = pe
        .resources()
        .map_err(|e| CheckError::PeParse(format!("no resource directory: {e}")))?;

    let version_info = resources
        .version_info()
        .map_err(|e| CheckError::PeParse(format!("no version info: {e}")))?;

    // Extract fixed file info for the version numbers
    let fixed = version_info
        .fixed()
        .ok_or_else(|| CheckError::PeParse("no VS_FIXEDFILEINFO".into()))?;

    let version = format!(
        "{}.{}.{}.{}",
        fixed.dwFileVersion.Major,
        fixed.dwFileVersion.Minor,
        fixed.dwFileVersion.Patch,
        fixed.dwFileVersion.Build,
    );

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: Some(url.to_string()),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    }))
}
