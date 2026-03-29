use astro_up_shared::version_file::VersionEntry;
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;

/// Aggregate all version files from `versions/{id}/{ver}.json` into the versions table.
/// Only imports versions for packages that exist in the packages table (skips orphans).
pub fn aggregate_versions(conn: &Connection, versions_dir: &Path) -> anyhow::Result<u64> {
    if !versions_dir.exists() {
        tracing::debug!("versions directory does not exist, skipping aggregation");
        return Ok(0);
    }

    // Get the set of known package IDs
    let mut stmt = conn.prepare("SELECT id FROM packages")?;
    let known_ids: HashSet<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let tx = conn.unchecked_transaction()?;
    let mut count = 0u64;

    for entry in std::fs::read_dir(versions_dir)? {
        let entry = entry?;
        let package_dir = entry.path();
        if !package_dir.is_dir() {
            continue;
        }

        let package_id = match package_dir.file_name().and_then(|n| n.to_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        // Skip orphaned version directories
        if !known_ids.contains(&package_id) {
            tracing::debug!("skipping orphaned version directory: {package_id}");
            continue;
        }

        for version_entry in std::fs::read_dir(&package_dir)? {
            let version_entry = version_entry?;
            let version_path = version_entry.path();

            if version_path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let version = version_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            match VersionEntry::read(&version_path) {
                Ok(ve) => {
                    tx.execute(
                        "INSERT OR REPLACE INTO versions (package_id, version, url, sha256, discovered_at, release_notes_url, pre_release)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            package_id,
                            version,
                            ve.url,
                            ve.sha256,
                            ve.discovered_at.to_rfc3339(),
                            ve.release_notes_url,
                            ve.pre_release as i32,
                        ],
                    )?;
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!("{}: {e}", version_path.display());
                }
            }
        }
    }

    tx.commit()?;
    tracing::info!("aggregated {count} version entries");
    Ok(count)
}
