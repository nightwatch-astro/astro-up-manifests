use astro_up_shared::manifest::{Checkver, Manifest};

use super::{CheckError, CheckOutcome, CheckResult};

pub fn check(manifest: &Manifest, checkver: &Checkver) -> Result<CheckOutcome, CheckError> {
    let version = checkver.version.as_deref().ok_or_else(|| {
        CheckError::Other(format!(
            "{}: static provider requires a 'version' field in [checkver]",
            manifest.id
        ))
    })?;

    tracing::info!("{}: static — version '{version}'", manifest.id);
    Ok(CheckOutcome::Found(CheckResult {
        version: version.into(),
        url: None,
        sha256: None,
        release_notes_url: None,
        pre_release: false,
        assets: Vec::new(),
    }))
}
