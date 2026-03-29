# Data Model: Manifest Pipeline

## Entities

### Manifest (TOML file)

Top-level metadata fields (required):

| Field | Type | Description |
|-------|------|-------------|
| id | string | Unique package identifier (e.g., `nina-app`) |
| manifest_version | integer | Schema version (starts at 1) |
| name | string | Human-readable name |
| description | string | Package description |
| publisher | string | Publisher/author name |
| homepage | string (URL) | Project homepage |
| category | string | Category: `capture`, `guiding`, `processing`, `planetarium`, `utility`, `driver` |
| type | string | Package type: `application`, `plugin`, `driver`, `library` |
| slug | string | URL-safe identifier |
| tags | string[] | Search tags |
| aliases | string[] | Alternative names |
| license | string | License identifier (optional) |

### Detection Section (optional)

| Field | Type | Description |
|-------|------|-------------|
| method | string | Detection method: `registry`, `file`, `pe_file`, `directory` |
| path | string | File/registry path for detection |
| registry_key | string | Windows registry key (if method = `registry`) |
| registry_value | string | Registry value name |
| file_version | boolean | Check PE file version (if method = `pe_file`) |
| fallback_path | string | Secondary detection path |
| fallback_method | string | Secondary detection method |

### Install Section (required)

| Field | Type | Description |
|-------|------|-------------|
| method | string | Installer type: `inno_setup`, `msi`, `nsis`, `zip_wrap`, `download_only`, `exe` |
| scope | string | Install scope: `user`, `machine`, `both` |
| elevation | boolean | Requires admin elevation |
| switches | map<string, string> | Override default installer switches (optional) |
| exit_codes | integer[] | Expected success exit codes beyond 0 |
| success_codes | integer[] | Additional success codes |

### Checkver Section (optional)

| Field | Type | Description |
|-------|------|-------------|
| provider | string | Check method: `github`, `gitlab`, `direct_url`, `http_head`, `html_scrape`, `browser_scrape`, `pe_download`, `manual` |
| owner | string | Repository owner (github/gitlab) |
| repo | string | Repository name (github/gitlab) |
| url | string (URL) | URL to check (direct_url, http_head, html_scrape, browser_scrape, pe_download) |
| regex | string | Regex to extract version from response |
| version_format | string | Version interpretation: `semver` (default), `date`, or custom regex |
| include_pre_release | boolean | Discover pre-release versions (default: false) |
| hash.url | string (URL) | URL to fetch hash from |
| hash.regex | string | Regex to extract hash |
| hash.jsonpath | string | JSONPath to extract hash |
| autoupdate | object (JSON) | Autoupdate URL templates with `$version` variables |

### Hardware Section (optional, drivers only)

| Field | Type | Description |
|-------|------|-------------|
| device_class | string | Windows device class |
| inf_provider | string | INF driver provider name |
| vid_pid | string[] | USB VID:PID patterns |

### Backup Section (optional)

| Field | Type | Description |
|-------|------|-------------|
| config_paths | string[] | Paths to back up (supports `$env` variables) |

### Dependencies Section (optional)

| Field | Type | Description |
|-------|------|-------------|
| requires | string[] | Package IDs this package depends on |

## SQLite Schema (catalog.db)

### packages

```sql
CREATE TABLE packages (
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
    tags TEXT,           -- JSON array
    aliases TEXT,        -- JSON array
    dependencies TEXT    -- JSON array
);

CREATE INDEX idx_packages_category ON packages(category);
CREATE INDEX idx_packages_type ON packages(type);
CREATE INDEX idx_packages_slug ON packages(slug);
```

### packages_fts (FTS5 virtual table)

```sql
CREATE VIRTUAL TABLE packages_fts USING fts5(
    name, description, tags, publisher,
    content='packages', content_rowid='rowid'
);
```

### detection

```sql
CREATE TABLE detection (
    package_id TEXT PRIMARY KEY REFERENCES packages(id),
    method TEXT NOT NULL,
    path TEXT,
    registry_key TEXT,
    registry_value TEXT,
    file_version INTEGER,
    fallback_path TEXT,
    fallback_method TEXT
);
```

### install

```sql
CREATE TABLE install (
    package_id TEXT PRIMARY KEY REFERENCES packages(id),
    method TEXT NOT NULL,
    scope TEXT,
    elevation INTEGER NOT NULL DEFAULT 0,
    switches TEXT,       -- JSON object
    exit_codes TEXT,     -- JSON array
    success_codes TEXT   -- JSON array
);
```

### checkver

```sql
CREATE TABLE checkver (
    package_id TEXT PRIMARY KEY REFERENCES packages(id),
    provider TEXT NOT NULL,
    owner TEXT,
    repo TEXT,
    url TEXT,
    regex TEXT,
    version_format TEXT DEFAULT 'semver',
    include_pre_release INTEGER NOT NULL DEFAULT 0,
    autoupdate TEXT,     -- JSON object
    hash TEXT            -- JSON object
);
```

### hardware

```sql
CREATE TABLE hardware (
    package_id TEXT PRIMARY KEY REFERENCES packages(id),
    device_class TEXT,
    inf_provider TEXT,
    vid_pid TEXT          -- JSON array
);
```

### backup

```sql
CREATE TABLE backup (
    package_id TEXT PRIMARY KEY REFERENCES packages(id),
    config_paths TEXT     -- JSON array
);
```

### versions

```sql
CREATE TABLE versions (
    package_id TEXT NOT NULL REFERENCES packages(id),
    version TEXT NOT NULL,
    url TEXT NOT NULL,
    sha256 TEXT,
    discovered_at TEXT NOT NULL,
    release_notes_url TEXT,
    pre_release INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (package_id, version)
);

CREATE INDEX idx_versions_package ON versions(package_id);
```

### meta

```sql
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Rows: schema_version, compiled_at
```

## State File (checker-state.json)

```json
{
  "nina-app": {
    "consecutive_failures": 0,
    "last_checked": "2026-03-29T12:00:00Z",
    "last_error": null,
    "issue_number": null
  }
}
```

## Version File (versions/{id}/{version}.json)

```json
{
  "url": "https://github.com/...",
  "sha256": "abc123...",
  "discovered_at": "2026-03-29T12:00:00Z",
  "release_notes_url": "https://github.com/.../releases/tag/v3.1.2",
  "pre_release": false
}
```
