# Feature Specification: Manifest Pipeline Modernization

**Feature Branch**: `001-manifest-pipeline`
**Created**: 2026-03-29
**Status**: Draft
**Project**: Rust Migration
**Project Number**: 1
**Input**: Migration plan Spec 019 — modernize the manifest repository with Scoop/winget-inspired patterns

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Self-Describing Manifest Format (Priority: P1)

A manifest author writes a TOML file for a new astrophotography application. The manifest contains both the software metadata (name, category, detection, install config) AND the version checking configuration (`[checkver]` section) in a single file. The `[checkver]` section uses Scoop-style `$version` template variables for URL construction and tiered hash discovery. The compiler reads this manifest and includes the checkver data in the compiled output — nothing is stripped.

**Why this priority**: The manifest format is the data contract that all other specs depend on. Every downstream consumer (catalog, providers, download, install) reads this format.

**Independent Test**: Write a manifest TOML for NINA with `[checkver]` section using `$version` variables. Compile it. Verify the compiled output retains the checkver configuration.

**Acceptance Scenarios**:

1. **Given** a manifest TOML with `[checkver]` section, **When** the compiler processes it, **Then** the checkver data is preserved in the compiled output (not stripped)
2. **Given** a manifest using `$version` in a URL template, **When** the version is `3.1.2`, **Then** `$version` resolves to `3.1.2`, `$majorVersion` to `3`, `$cleanVersion` to `312`, `$underscoreVersion` to `3_1_2`, `$dashVersion` to `3-1-2`
3. **Given** a manifest with `hash.url` and `hash.regex`, **When** the checker runs, **Then** it fetches the hash URL and extracts the SHA256 using the regex pattern

---

### User Story 2 - TOML to SQLite Compilation (Priority: P2)

A CI pipeline compiles 95+ TOML manifests into a single database file. The database contains all manifest data plus discovered version information, enabling the client app to query software by ID, category, name, or fuzzy search without JSON parsing. The compiled artifact is signed and published as a release asset.

**Why this priority**: The compiled database replaces the current `manifests.json` + `versions.json` dual-file approach, simplifying the client's data access pattern.

**Independent Test**: Run the compiler against the manifests directory. Verify the output contains all 95 manifests queryable by ID and category.

**Acceptance Scenarios**:

1. **Given** a directory of TOML manifests, **When** the compiler runs, **Then** it produces a compiled database file containing all manifest data
2. **Given** the compiled database, **When** querying by category "capture", **Then** it returns all capture software entries with their full metadata
3. **Given** a new version discovered for a package, **When** the compiler runs, **Then** the version data from per-version files is imported into the database
4. **Given** the compiled database, **When** signing with minisign, **Then** it produces a verifiable signature file

---

### User Story 3 - Per-Version File Storage (Priority: P3)

When the CI version checker discovers a new version of a package, it writes a JSON file at `versions/{package-id}/{semver}.json` containing the download URL, SHA256 hash, discovery timestamp, and release notes URL. Git history provides the full audit trail. The compiler aggregates all per-version files into the compiled database.

**Why this priority**: Per-version files replace the flat `versions.json`, enabling granular version history with git as the audit trail and supporting rollback to any previously-discovered version.

**Independent Test**: Create a per-version file for NINA 3.1.2. Run the compiler. Verify the version appears in the database with correct URL and hash.

**Acceptance Scenarios**:

1. **Given** the checker discovers NINA version 3.1.2, **When** it writes the version file, **Then** `versions/nina-app/3.1.2.json` contains `{ "url": "...", "sha256": "...", "discovered_at": "...", "release_notes_url": "...", "pre_release": false }`
4. **Given** the checker discovers NINA version 3.2.0-rc1, **When** it writes the version file, **Then** `versions/nina-app/3.2.0-rc1.json` contains `"pre_release": true` and the version is stored alongside stable versions
2. **Given** multiple version files exist for a package, **When** the compiler runs, **Then** the database contains all versions ordered by semver
3. **Given** a version file was written 6 months ago, **When** querying the git log for that file, **Then** the discovery date and commit context are available as audit trail

---

### User Story 4 - Automated Version Checking (Priority: P4)

The version checker iterates all manifests and discovers new versions using the `[checkver]` configuration in each manifest. It supports 9 check methods covering GitHub releases, GitLab tags, direct URL polling, HTML scraping, browser-based scraping, and PE file header inspection. Failed checks retry with exponential backoff. Rate-limited responses are respected by pausing that provider.

**Why this priority**: Version checking is the core automation that populates the version files. Without it, versions must be added manually.

**Independent Test**: Run the checker against a subset of manifests. Verify it discovers the latest versions and writes correct version files.

**Acceptance Scenarios**:

1. **Given** a manifest with `checkver.provider = "github"`, **When** the checker runs, **Then** it queries the GitHub API for the latest release and writes a version file
2. **Given** a manifest with `checkver.provider = "html_scrape"`, **When** the checker runs, **Then** it fetches the URL and extracts the version using the configured regex
3. **Given** a vendor website returns HTTP 429, **When** the checker encounters the rate limit, **Then** it pauses checks for that provider until the reset window and continues with other providers
4. **Given** a manifest with `checkver.provider = "browser_scrape"`, **When** the checker runs, **Then** it launches a headless browser, loads the page, and extracts the version from the rendered DOM

---

### User Story 5 - Simplified CI Pipeline (Priority: P5)

The version checking and compilation pipeline runs as a single CI job on a 6-hour cron schedule. It iterates all manifests, runs checkver for each, writes new version files, compiles to the database, signs the artifact, and publishes it as a release asset on a rolling `catalog/latest` tag.

**Why this priority**: Replacing the current 3-job matrix (resolve → check → merge) with a single job reduces CI complexity and cost.

**Independent Test**: Trigger the CI workflow manually. Verify it checks versions, compiles, signs, and publishes the database + signature assets.

**Acceptance Scenarios**:

1. **Given** the CI runs on schedule, **When** a new version is discovered, **Then** the version file is written, database is recompiled, signed, and uploaded to the `catalog/latest` release
2. **Given** a vendor website is unreachable, **When** the checker runs, **Then** it logs the failure and continues to the next manifest (no pipeline abort)
3. **Given** no new versions are discovered, **When** the pipeline completes, **Then** no new commit is created and the release assets remain unchanged
4. **Given** the `catalog/latest` release exists, **When** the pipeline uploads new assets, **Then** the old assets are replaced (clobber mode)

---

### Edge Cases

- What happens when a manifest TOML has invalid syntax? The compiler MUST report the error with file path and line number, skip the invalid manifest, and continue processing.
- What happens when a `$version` variable is used but the version string has no minor component (e.g., "3")? The variable MUST resolve to empty string for missing components (`$minorVersion` = "", `$patchVersion` = "").
- What happens when the SHA256 hash from `hash.url` doesn't match the downloaded file? The version file MUST NOT be written. The mismatch MUST be logged as an error and an issue auto-created.
- What happens when the `catalog/latest` release doesn't exist on first run? The pipeline MUST create it before uploading assets.
- What happens when the manifest repo has 200+ manifests? The pipeline MUST complete within 15 minutes on a standard CI runner.
- What happens when a GitHub App token expires mid-run? The checker MUST refresh the token transparently and retry the failed request.
- What happens when two checker instances run concurrently (manual + cron overlap)? The CI MUST use concurrency groups to prevent parallel runs.
- What happens when a manifest is deleted but version files remain in `versions/{id}/`? The compiler MUST ignore version files without a matching manifest. Orphaned files remain in git history but are excluded from the compiled database.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each TOML manifest MUST include a `manifest_version` field for schema versioning (starting at `1`). The compiler MUST declare which manifest versions it supports and skip manifests with unsupported versions with a clear error. Required sections: top-level metadata fields and `[install]`. Optional sections: `[detection]`, `[checkver]`, `[hardware]`, `[backup]`, `[dependencies]`
- **FR-002**: The `[checkver]` section contains version checking configuration. The checkver data MUST be preserved in the compiled output (not stripped)
- **FR-003**: URL templates in `[checkver]` and `[checkver.autoupdate]` MUST support Scoop-style variable substitution: `$version`, `$majorVersion`, `$minorVersion`, `$patchVersion`, `$cleanVersion`, `$underscoreVersion`, `$dashVersion`, `$preReleaseVersion`, `$buildVersion`
- **FR-004**: Hash discovery MUST support three methods, determined by which fields are present in the manifest (one method per manifest, not a fallback chain): `hash.url` + `hash.regex` — fetch URL, extract hash via regex; `hash.jsonpath` — fetch JSON, extract via path; or download the file and compute SHA256 (default when no hash source is configured)
- **FR-005**: Default installer switches MUST be defined per installer type (InnoSetup: `/VERYSILENT /NORESTART /SUPPRESSMSGBOXES`, MSI: `/qn /norestart`, etc.) to reduce manifest verbosity. Manifests MAY override defaults
- **FR-006**: The compiler MUST produce a SQLite database (`catalog.db`) with a pragmatic normalized schema (8 tables): `packages` (metadata columns + JSON for tags, aliases, dependencies), `detection` (method + method-specific columns, fallback fields flattened), `install` (method, scope, elevation + JSON for switches, exit_codes, success_codes), `checkver` (provider, owner, repo, url, regex, version_format + JSON for autoupdate, hash), `hardware` (device_class, inf_provider + JSON for vid_pid), `backup` (JSON for config_paths), `versions` (package_id + version as PK, url, sha256, discovered_at, release_notes_url, pre_release), `meta` (key-value for schema version and compilation timestamp). Indexes on `packages.category`, `packages.type`, `packages.slug`. FTS5 virtual table on `name`, `description`, `tags`, `publisher` for fuzzy search
- **FR-007**: The compiler MUST produce a `catalog.db.minisig` signature file using the CI's minisign private key
- **FR-008**: Discovered versions MUST be stored as individual JSON files at `versions/{package-id}/{version}.json` with fields: `url`, `sha256`, `discovered_at`, `release_notes_url`, `pre_release` (boolean, true for versions matching pre-release semver patterns like `-rc`, `-beta`, `-alpha`)
- **FR-009**: The CI pipeline MUST run as a single job: iterate manifests → checkver → write version files → compile SQLite → sign → publish to GitHub Releases (`catalog/latest` tag, clobber mode). Publish MUST retry 3 times with backoff on failure; if all retries fail, the job MUST exit with error
- **FR-010**: The CI pipeline MUST run on a 6-hour cron schedule and on manual dispatch with an optional filter parameter (substring match on package ID, category, or provider name)
- **FR-011**: The pipeline MUST auto-create GitHub issues when a vendor check fails for 8 consecutive pipeline runs (~2 days) and auto-close when the check succeeds again. Failure counts MUST be persisted in a committed state file (`checker-state.json`) updated each pipeline run. A single successful check MUST auto-close the issue and reset the failure count
- **FR-013**: The manifest TOML format MUST include a `[hardware]` section for driver packages: `vid_pid` (USB VID:PID patterns), `device_class`, `inf_provider`
- **FR-014**: The compiler and checker MUST be implemented as separate binaries in the manifest repository (replacing the current Go modules)
- **FR-015**: The checker MUST support all existing check methods:
  - `github`: Query GitHub Releases API for the latest release. Pre-releases are only discovered when `checkver.include_pre_release = true`
  - `gitlab`: Query GitLab Tags API for the latest tag. Pre-releases are only discovered when `checkver.include_pre_release = true`
  - `direct_url`: Fetch `checkver.url`, extract version from response body using `checkver.regex`
  - `http_head`: Send HTTP HEAD to `checkver.url`, extract version from response headers (Content-Disposition, Location redirect) using `checkver.regex`
  - `html_scrape`: Fetch HTML page at `checkver.url`, extract version using `checkver.regex` against the page body
  - `browser_scrape`: Launch headless browser, load `checkver.url` (60-second page load timeout), wait for DOM rendering (30-second extraction timeout), extract version using `checkver.regex` against rendered page. On timeout, treat as a failed check
  - `pe_download`: Download the installer executable from `checkver.url`, read the `FileVersion` field from the PE (Portable Executable) header
  - `manual`: Skip the check and log "manual: requires human update" in the pipeline output
- **FR-018**: Each manifest MAY include a `checkver.version_format` field to declare how version strings are interpreted. Supported values: `semver` (default when omitted — parsed as semver, ordered by semver rules, pre-release detection enabled), `date` (parsed as date, ordered chronologically), or a custom regex with named capture groups for component extraction and ordering. Version file paths MUST sanitize the version string for filesystem safety (replace unsafe characters)
- **FR-016**: The checker MUST run checks in parallel (configurable concurrency, default 10) using authenticated requests. Failed checks MUST retry with exponential backoff (3 attempts, 1s/2s/4s). Rate limit responses (HTTP 429, `retry-after`) MUST be respected by pausing that provider until the reset window
- **FR-017**: Manifests MUST be validated at compilation time via typed struct deserialization with declarative constraints. Invalid manifests MUST be skipped with a clear error (file path + field name), not abort the entire compilation. CI MUST validate manifests on every PR via the compiler in dry-run mode (`--validate`)

### Key Entities

- **Manifest**: A TOML file defining a software package — metadata, detection, install, checkver, hardware, backup config
- **Version Entry**: A JSON file recording a discovered version — URL, SHA256, timestamp, release notes link
- **Catalog Database**: The compiled artifact containing all manifests + versions, served via releases
- **Check Method**: A strategy for discovering the latest version of a package (API query, HTML scraping, PE download, etc.)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 95 astrophotography packages are represented in the new manifest format with complete metadata, detection, install, and checkver data
- **SC-002**: The compiled database is smaller than or equal to 200 KB (FTS5 index adds overhead beyond raw data size)
- **SC-003**: The single-job CI pipeline completes in under 15 minutes for all 95 manifests
- **SC-004**: The client can fetch the database via ETag conditional request and skip download when unchanged
- **SC-005**: Version discovery accurately finds the latest version for all manifests with automated check methods

## Assumptions

- The manifest repository is hosted at `nightwatch-astro/astro-up-manifests`
- The minisign private key is stored as a repository secret
- The GitHub App token (`NIGHTWATCH_APP_ID` + `NIGHTWATCH_APP_PRIVATE_KEY`) is available for CI operations
- Browser-based scraping (`browser_scrape`) uses a headless browser dependency isolated in the checker binary — not shipped to clients
- The catalog database schema is versioned separately from the manifest schema — breaking changes to the DB schema require a version bump
- The client app (separate spec) handles fetching and querying the compiled database
- The 95 existing packages in the old repository serve as a data reference for porting (greenfield format, not a migration)

## Clarifications

### Session 2026-03-29

- Q: SQLite catalog schema — normalized, hybrid, or denormalized? → A: Pragmatic normalized (Option D). 8 tables: `packages`, `detection`, `install`, `checkver`, `hardware`, `backup`, `versions`, `meta`. Top-level config fields are proper columns; only arrays/maps (switches, exit_codes, tags, aliases, vid_pid, config_paths, autoupdate, hash) stay as JSON. FTS5 for fuzzy search. Revised from initial hybrid (Option B) after deeper analysis — JSON-only columns underuse SQLite.
- Q: GitHub API rate limiting strategy for CI checker? → A: Parallel checks (configurable concurrency, default 10) with authenticated GitHub App token. Exponential backoff retries (3 attempts, 1s/2s/4s). Respect HTTP 429 and retry-after headers.
- Q: Where do the checker/compiler binaries live? → A: Separate Cargo workspace in the manifest repo. Independent CI, own dependencies, no coupling to main app workspace. Browser dependency stays isolated.
- Q: Browser scraping strategy for checker? → A: `chromiumoxide` crate — modern async CDP client, tokio-native. Replaces Go Rod. CI caches the browser binary.
- Q: Manifest validation strategy? → A: Typed structs with serde + garde. Validated at compile time (compiler binary run, not language compilation). Invalid manifests skipped with clear errors, never abort entire compilation. CI validates on PR via `--validate` flag.
- Q: What defines "persistent failure" for auto-issue creation (FR-011)? → A: 8 consecutive failures across separate pipeline runs (~2 days at 6h cron interval). Filters transient outages and weekend maintenance while catching genuine breakage within a reasonable window.
- Q: Should the checker discover and store pre-release versions? → A: Yes — store pre-releases with a `pre_release: true` boolean field in the version JSON and the versions table. Downstream clients can filter on this field. Astrophotography users often want to test release candidates (NINA betas are popular).
- Q: How to handle non-semver version strings (date-based, integers, custom)? → A: Add `checkver.version_format` field to manifest: `semver` (default), `date`, or custom regex. Determines parsing, ordering, and pre-release detection. Version file paths sanitize the version string for filesystem safety.
