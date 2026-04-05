use astro_up_shared::manifest::Manifest;
use astro_up_shared::validate::{apply_default_switches, validate_manifest};
use std::path::Path;

/// Result of loading manifests from a directory.
pub struct LoadResult {
    pub manifests: Vec<Manifest>,
    pub errors: Vec<String>,
}

/// Load all `.toml` manifest files from a directory.
/// Invalid manifests are skipped with errors collected (not fatal).
///
/// # Errors
///
/// Returns an error if the manifests directory does not exist or cannot be read.
pub fn load_manifests(dir: &Path) -> anyhow::Result<LoadResult> {
    let mut manifests = Vec::new();
    let mut errors = Vec::new();

    if !dir.exists() {
        anyhow::bail!("manifests directory does not exist: {}", dir.display());
    }

    for entry in walkdir::WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let file_name = path.display().to_string();
        match load_single_manifest(path) {
            Ok(manifest) => {
                let validation_errors = validate_manifest(&manifest, &file_name);
                if validation_errors.is_empty() {
                    manifests.push(manifest);
                } else {
                    for err in validation_errors {
                        tracing::warn!("{err}");
                        errors.push(err.to_string());
                    }
                }
            }
            Err(e) => {
                let msg = format!("{file_name}: {e}");
                tracing::warn!("{msg}");
                errors.push(msg);
            }
        }
    }

    // Apply default installer switches to all valid manifests
    for manifest in &mut manifests {
        apply_default_switches(&mut manifest.install);
    }

    tracing::info!(
        "loaded {} manifests ({} errors)",
        manifests.len(),
        errors.len()
    );

    Ok(LoadResult { manifests, errors })
}

fn load_single_manifest(path: &Path) -> anyhow::Result<Manifest> {
    let content = std::fs::read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&content)?;
    Ok(manifest)
}
