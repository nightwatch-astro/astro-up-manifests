use astro_up_shared::manifest::{Checkver, Manifest};
use reqwest_middleware::ClientWithMiddleware;

use super::{CheckError, CheckOutcome, CheckResult};

pub async fn check(
    _manifest: &Manifest,
    checkver: &Checkver,
    client: &ClientWithMiddleware,
) -> Result<CheckOutcome, CheckError> {
    let url = checkver.url.as_deref()
        .ok_or_else(|| CheckError::MissingConfig("url".into()))?;

    // Download the executable
    let bytes = client.get(url).send().await?.bytes().await?;

    // Parse PE headers using goblin
    let pe = goblin::pe::PE::parse(&bytes)
        .map_err(|e| CheckError::PeParse(format!("{e}")))?;

    // Extract version from optional header — major/minor linker version
    // as a best-effort. Real VS_VERSION_INFO parsing requires walking
    // the resource directory which goblin doesn't directly expose.
    let version = if let Some(header) = pe.header.optional_header {
        let win = header.windows_fields;
        format!(
            "{}.{}.{}.{}",
            win.major_image_version,
            win.minor_image_version,
            win.major_operating_system_version,
            win.minor_operating_system_version,
        )
    } else {
        return Err(CheckError::PeParse("no optional header in PE".into()));
    };

    Ok(CheckOutcome::Found(CheckResult {
        version,
        url: Some(url.to_string()),
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    }))
}
