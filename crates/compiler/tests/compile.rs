use astro_up_compiler::{compile, manifest, schema, version_file};
use astro_up_shared::version_file::VersionEntry;
use chrono::Utc;
use rusqlite::Connection;
use std::path::Path;

fn compile_sample_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test-catalog.db");

    let manifests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = manifest::load_manifests(&manifests_dir).unwrap();
    assert!(!result.manifests.is_empty());

    let conn = Connection::open(&db_path).unwrap();
    schema::create_schema(&conn).unwrap();
    compile::compile_manifests(&conn, &result.manifests).unwrap();

    (dir, conn)
}

#[test]
fn compile_produces_packages() {
    let (_dir, conn) = compile_sample_db();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM packages", [], |row| row.get(0))
        .unwrap();
    assert!(count >= 4, "expected at least 4 packages, got {count}");
}

#[test]
fn query_by_id() {
    let (_dir, conn) = compile_sample_db();

    let name: String = conn
        .query_row(
            "SELECT name FROM packages WHERE id = ?1",
            ["nina-app"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "N.I.N.A.");
}

#[test]
fn query_by_category() {
    let (_dir, conn) = compile_sample_db();

    let mut stmt = conn
        .prepare("SELECT id FROM packages WHERE category = ?1")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map(["capture"], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(ids.contains(&"nina-app".to_string()));
    assert!(ids.contains(&"sharpcap-app".to_string()));
}

#[test]
fn fts5_search() {
    let (_dir, conn) = compile_sample_db();

    let mut stmt = conn
        .prepare("SELECT id FROM packages WHERE rowid IN (SELECT rowid FROM packages_fts WHERE packages_fts MATCH ?1)")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map(["autoguiding"], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(ids.contains(&"phd2-app".to_string()));
}

#[test]
fn checkver_preserved() {
    let (_dir, conn) = compile_sample_db();

    let provider: String = conn
        .query_row(
            "SELECT provider FROM checkver WHERE package_id = ?1",
            ["nina-app"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(provider, "html_scrape");

    let autoupdate: String = conn
        .query_row(
            "SELECT autoupdate FROM checkver WHERE package_id = ?1",
            ["nina-app"],
            |row| row.get(0),
        )
        .unwrap();
    assert!(autoupdate.contains("backblazeb2"));
}

#[test]
fn hardware_compiled() {
    let (_dir, conn) = compile_sample_db();

    let device_class: String = conn
        .query_row(
            "SELECT device_class FROM hardware WHERE package_id = ?1",
            ["zwo-asi-driver"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(device_class, "Camera");
}

#[test]
fn meta_table_populated() {
    let (_dir, conn) = compile_sample_db();

    let schema_version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(schema_version, "1");

    let compiled_at: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'compiled_at'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!compiled_at.is_empty());
}

#[test]
fn version_aggregation() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test-catalog.db");
    let versions_dir = dir.path().join("versions");

    // Create a version file
    let nina_versions = versions_dir.join("nina-app");
    std::fs::create_dir_all(&nina_versions).unwrap();
    let entry = VersionEntry {
        url: "https://example.com/nina-3.1.2.exe".into(),
        sha256: Some("abc123".into()),
        discovered_at: Utc::now(),
        release_notes_url: Some("https://example.com/release".into()),
        pre_release: false,
    };
    entry
        .write(&nina_versions.join("3.1.2.json"))
        .unwrap();

    // Compile with versions
    let manifests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = manifest::load_manifests(&manifests_dir).unwrap();

    let conn = Connection::open(&db_path).unwrap();
    schema::create_schema(&conn).unwrap();
    compile::compile_manifests(&conn, &result.manifests).unwrap();
    let count = version_file::aggregate_versions(&conn, &versions_dir).unwrap();

    assert_eq!(count, 1);

    let url: String = conn
        .query_row(
            "SELECT url FROM versions WHERE package_id = ?1 AND version = ?2",
            ["nina-app", "3.1.2"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(url, "https://example.com/nina-3.1.2.exe");
}

#[test]
fn orphaned_versions_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test-catalog.db");
    let versions_dir = dir.path().join("versions");

    // Create version file for a package that doesn't exist
    let orphan_dir = versions_dir.join("nonexistent-pkg");
    std::fs::create_dir_all(&orphan_dir).unwrap();
    let entry = VersionEntry {
        url: "https://example.com/orphan.exe".into(),
        sha256: None,
        discovered_at: Utc::now(),
        release_notes_url: None,
        pre_release: false,
    };
    entry.write(&orphan_dir.join("1.0.0.json")).unwrap();

    let manifests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
    let result = manifest::load_manifests(&manifests_dir).unwrap();

    let conn = Connection::open(&db_path).unwrap();
    schema::create_schema(&conn).unwrap();
    compile::compile_manifests(&conn, &result.manifests).unwrap();
    let count = version_file::aggregate_versions(&conn, &versions_dir).unwrap();

    assert_eq!(count, 0, "orphaned version files should be skipped");
}
