use rusqlite::Connection;

/// Create all tables, indexes, and FTS5 virtual table in the catalog database.
pub fn create_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS packages (
            id TEXT PRIMARY KEY,
            manifest_version INTEGER NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            publisher TEXT,
            homepage TEXT,
            category TEXT NOT NULL,
            type TEXT NOT NULL,
            slug TEXT NOT NULL,
            license TEXT,
            tags TEXT,
            aliases TEXT,
            dependencies TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_packages_category ON packages(category);
        CREATE INDEX IF NOT EXISTS idx_packages_type ON packages(type);
        CREATE INDEX IF NOT EXISTS idx_packages_slug ON packages(slug);

        CREATE TABLE IF NOT EXISTS detection (
            package_id TEXT PRIMARY KEY REFERENCES packages(id),
            method TEXT NOT NULL,
            path TEXT,
            registry_key TEXT,
            registry_value TEXT,
            file_version INTEGER,
            fallback_path TEXT,
            fallback_method TEXT
        );

        CREATE TABLE IF NOT EXISTS install (
            package_id TEXT PRIMARY KEY REFERENCES packages(id),
            method TEXT NOT NULL,
            scope TEXT,
            elevation INTEGER NOT NULL DEFAULT 0,
            switches TEXT,
            exit_codes TEXT,
            success_codes TEXT
        );

        CREATE TABLE IF NOT EXISTS checkver (
            package_id TEXT PRIMARY KEY REFERENCES packages(id),
            provider TEXT NOT NULL,
            owner TEXT,
            repo TEXT,
            url TEXT,
            regex TEXT,
            version_format TEXT DEFAULT 'semver',
            include_pre_release INTEGER NOT NULL DEFAULT 0,
            autoupdate TEXT,
            hash TEXT
        );

        CREATE TABLE IF NOT EXISTS hardware (
            package_id TEXT PRIMARY KEY REFERENCES packages(id),
            device_class TEXT,
            inf_provider TEXT,
            vid_pid TEXT
        );

        CREATE TABLE IF NOT EXISTS backup (
            package_id TEXT PRIMARY KEY REFERENCES packages(id),
            config_paths TEXT
        );

        CREATE TABLE IF NOT EXISTS versions (
            package_id TEXT NOT NULL REFERENCES packages(id),
            version TEXT NOT NULL,
            url TEXT NOT NULL,
            sha256 TEXT,
            discovered_at TEXT NOT NULL,
            release_notes_url TEXT,
            pre_release INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (package_id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_versions_package ON versions(package_id);

        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS packages_fts USING fts5(
            name, description, tags, aliases, publisher,
            content='packages', content_rowid='rowid'
        );
        ",
    )?;

    Ok(())
}
