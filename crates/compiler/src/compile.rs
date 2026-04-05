use astro_up_shared::manifest::Manifest;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use rusqlite::{Connection, params};
use std::path::Path;

const SCHEMA_VERSION: &str = "1";

/// Compile a list of manifests into the `SQLite` database.
/// Assumes schema is already created. Runs within a transaction.
///
/// # Errors
///
/// Returns an error if any database operation fails.
pub fn compile_manifests(
    conn: &Connection,
    manifests: &[Manifest],
    icons_dir: &Path,
) -> anyhow::Result<()> {
    let tx = conn.unchecked_transaction()?;

    for manifest in manifests {
        let icon_base64 = resolve_icon(manifest, icons_dir);
        insert_package(&tx, manifest, icon_base64.as_deref())?;

        if let Some(detection) = &manifest.detection {
            insert_detection(&tx, &manifest.id, detection)?;
        }

        insert_install(&tx, &manifest.id, &manifest.install)?;

        if let Some(checkver) = &manifest.checkver {
            insert_checkver(&tx, &manifest.id, checkver)?;
        }

        if let Some(hardware) = &manifest.hardware {
            insert_hardware(&tx, &manifest.id, hardware)?;
        }

        if let Some(backup) = &manifest.backup {
            insert_backup(&tx, &manifest.id, backup)?;
        }
    }

    // Populate FTS5 index
    tx.execute_batch(
        "INSERT INTO packages_fts(rowid, name, description, tags, aliases, publisher)
         SELECT rowid, name, description, tags, aliases_normalized, publisher FROM packages;",
    )?;

    // Write meta
    tx.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        params!["schema_version", SCHEMA_VERSION],
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        params!["compiled_at", Utc::now().to_rfc3339()],
    )?;

    tx.commit()?;
    Ok(())
}

/// Resolve icon for a manifest. Lookup order:
/// 1. Explicit `icon` field in manifest → `{icons_dir}/{icon}.png`
/// 2. Publisher slug fallback → `{icons_dir}/{publisher_slug}.png`
///
/// Returns raw base64 string (no data URI prefix) or None.
fn resolve_icon(manifest: &Manifest, icons_dir: &Path) -> Option<String> {
    let candidates: Vec<std::path::PathBuf> = [
        // 1. Explicit icon field
        manifest
            .icon
            .as_deref()
            .map(|i| icons_dir.join(format!("{i}.png"))),
        // 2. Publisher slug (lowercase, spaces → hyphens)
        manifest.publisher.as_deref().map(|p| {
            let slug = p.to_lowercase().replace(' ', "-");
            icons_dir.join(format!("{slug}.png"))
        }),
    ]
    .into_iter()
    .flatten()
    .collect();

    for path in &candidates {
        if path.is_file() {
            match std::fs::read(path) {
                Ok(bytes) => {
                    tracing::debug!("icon for {}: {}", manifest.id, path.display());
                    return Some(STANDARD.encode(bytes));
                }
                Err(e) => {
                    tracing::warn!("failed to read icon {}: {e}", path.display());
                }
            }
        }
    }

    tracing::debug!("no icon found for {}", manifest.id);
    None
}

fn insert_package(
    conn: &Connection,
    manifest: &Manifest,
    icon_base64: Option<&str>,
) -> rusqlite::Result<usize> {
    // Normalize aliases for FTS5 indexing: strip dots and hyphens, join with spaces
    let aliases_normalized = if manifest.aliases.is_empty() {
        None
    } else {
        Some(
            manifest
                .aliases
                .iter()
                .map(|alias| alias.replace(['.', '-'], ""))
                .collect::<Vec<_>>()
                .join(" "),
        )
    };

    conn.execute(
        "INSERT INTO packages (id, manifest_version, name, description, publisher, homepage, category, type, slug, license, tags, aliases, aliases_normalized, dependencies, icon_base64)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            manifest.id,
            manifest.manifest_version,
            manifest.name,
            manifest.description,
            manifest.publisher,
            manifest.homepage,
            manifest.category,
            manifest.package_type,
            manifest.slug,
            manifest.license,
            serde_json::to_string(&manifest.tags).ok(),
            serde_json::to_string(&manifest.aliases).ok(),
            aliases_normalized,
            manifest.dependencies.as_ref().and_then(|d| serde_json::to_string(&d.requires).ok()),
            icon_base64,
        ],
    )
}

fn insert_detection(
    conn: &Connection,
    package_id: &str,
    detection: &astro_up_shared::manifest::Detection,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO detection (package_id, method, path, registry_key, registry_value, file_version, fallback_path, fallback_method)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            package_id,
            detection.method,
            detection.path,
            detection.registry_key,
            detection.registry_value,
            detection.file_version.map(i32::from),
            detection.fallback_path,
            detection.fallback_method,
        ],
    )
}

fn insert_install(
    conn: &Connection,
    package_id: &str,
    install: &astro_up_shared::manifest::Install,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO install (package_id, method, scope, elevation, switches, exit_codes, success_codes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            package_id,
            install.method,
            install.scope,
            i32::from(install.elevation),
            serde_json::to_string(&install.switches).ok(),
            serde_json::to_string(&install.exit_codes).ok(),
            serde_json::to_string(&install.success_codes).ok(),
        ],
    )
}

fn insert_checkver(
    conn: &Connection,
    package_id: &str,
    checkver: &astro_up_shared::manifest::Checkver,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO checkver (package_id, provider, owner, repo, url, regex, version_format, include_pre_release, autoupdate, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            package_id,
            checkver.provider,
            checkver.owner,
            checkver.repo,
            checkver.url,
            checkver.regex,
            checkver.version_format,
            i32::from(checkver.include_pre_release),
            checkver.autoupdate.as_ref().and_then(|a| serde_json::to_string(a).ok()),
            checkver.hash.as_ref().and_then(|h| serde_json::to_string(h).ok()),
        ],
    )
}

fn insert_hardware(
    conn: &Connection,
    package_id: &str,
    hardware: &astro_up_shared::manifest::Hardware,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO hardware (package_id, device_class, inf_provider, vid_pid)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            package_id,
            hardware.device_class,
            hardware.inf_provider,
            serde_json::to_string(&hardware.vid_pid).ok(),
        ],
    )
}

fn insert_backup(
    conn: &Connection,
    package_id: &str,
    backup: &astro_up_shared::manifest::Backup,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO backup (package_id, config_paths) VALUES (?1, ?2)",
        params![package_id, serde_json::to_string(&backup.config_paths).ok(),],
    )
}
