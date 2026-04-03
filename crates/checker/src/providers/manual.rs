use astro_up_shared::manifest::Manifest;

use super::{CheckOutcome, CheckResult};

pub fn check(manifest: &Manifest) -> CheckOutcome {
    tracing::info!("{}: manual — writing 'latest' as version", manifest.id);
    CheckOutcome::Found(CheckResult {
        version: "latest".into(),
        url: None,
        sha256: None,
        release_notes_url: None,
        pre_release: false,
    })
}
