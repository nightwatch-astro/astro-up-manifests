use astro_up_shared::manifest::Manifest;

use super::CheckOutcome;

pub fn check(manifest: &Manifest) -> CheckOutcome {
    tracing::info!("{}: manual — requires human update", manifest.id);
    CheckOutcome::Skipped {
        reason: "manual: requires human update".into(),
    }
}
