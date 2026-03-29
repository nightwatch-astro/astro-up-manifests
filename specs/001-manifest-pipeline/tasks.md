# Tasks: Manifest Pipeline Modernization

**Input**: Design documents from `/specs/001-manifest-pipeline/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Cargo workspace, shared crate, and project scaffolding

- [x] T001 Create `crates/shared/Cargo.toml` with dependencies: serde, serde_json, toml, semver, lenient_semver, chrono, regex, tracing
- [x] T002 Create `crates/compiler/Cargo.toml` with dependencies: shared (path), rusqlite (bundled), clap (derive), tracing-subscriber
- [x] T003 Create `crates/checker/Cargo.toml` with dependencies: shared (path), reqwest (json), reqwest-middleware, reqwest-retry, tokio (full), futures, scraper, chromiumoxide, goblin, sha2, clap (derive), tracing-subscriber
- [x] T004 Update root `Cargo.toml` workspace members to add `crates/shared` alongside existing `crates/compiler` and `crates/checker` entries
- [x] T005 Create `crates/compiler/src/main.rs` with clap CLI skeleton (--manifests, --versions, --output, --validate, --verbose)
- [x] T006 Create `crates/checker/src/main.rs` with clap + tokio CLI skeleton (--manifests, --versions, --state, --concurrency, --filter, --verbose)

---

## Phase 2: Foundational (Shared Crate)

**Purpose**: Manifest types and template engine used by both compiler and checker. MUST complete before any user story.

- [x] T007 Define manifest TOML types in `crates/shared/src/manifest.rs`: top-level metadata struct with all required/optional fields, nested structs for Detection, Install, Checkver, Hardware, Backup, Dependencies sections. Include serde Deserialize/Serialize derives
- [x] T008 [P] Implement custom validation functions in `crates/shared/src/validate.rs`: validate manifest_version is supported, validate required fields (id, name, category, type, slug), validate install method is known (inno_setup, msi, nsis, zip_wrap, download_only, exe), validate checkver provider is known, validate URL fields are valid. Include default installer switches per method (FR-005) that are applied when manifest omits switches
- [x] T009 [P] Implement `$version` template variable substitution in `crates/shared/src/template.rs`: parse `$version`, `$majorVersion`, `$minorVersion`, `$patchVersion`, `$cleanVersion`, `$underscoreVersion`, `$dashVersion`, `$preReleaseVersion`, `$buildVersion` from version strings
- [x] T010 [P] Implement version parsing in `crates/shared/src/version.rs`: parse semver (via lenient_semver), parse date format, parse custom regex format. Implement ordering for each format type. Handle `version_format` field dispatch
- [x] T011 [P] Define version file types in `crates/shared/src/version_file.rs`: VersionEntry struct (url, sha256, discovered_at, release_notes_url, pre_release), read/write JSON
- [x] T012 [P] Define checker state types in `crates/shared/src/state.rs`: CheckerState struct with per-manifest failure count, last_checked, last_error, issue_number. Read/write JSON
- [x] T013 Create `crates/shared/src/lib.rs` re-exporting all public types and modules
- [x] T014 Add integration test `crates/shared/tests/manifest_parse.rs`: deserialize a sample TOML manifest, verify all fields round-trip correctly
- [x] T015 [P] Add integration test `crates/shared/tests/template_substitution.rs`: verify all `$version` variables resolve correctly for version "3.1.2", test edge cases (missing minor, pre-release, build metadata)

**Checkpoint**: Shared crate complete — compiler and checker implementation can begin

---

## Phase 3: User Story 1 — Self-Describing Manifest Format (Priority: P1) MVP

**Goal**: TOML manifests with `[checkver]` section are parsed, validated, and their data preserved through compilation

**Independent Test**: Write a NINA manifest TOML with `[checkver]` using `$version` variables. Compile it. Verify compiled output retains checkver data.

- [x] T016 [US1] Create sample manifest `manifests/nina-app.toml` with all sections populated (metadata, detection, install, checkver with github provider and $version templates, hardware: none, backup with config paths)
- [x] T017 [P] [US1] Create 2-3 additional sample manifests in `manifests/`: one with `html_scrape` provider, one `manual` provider, one driver with `[hardware]` section
- [x] T018 [US1] Implement manifest loading in `crates/compiler/src/manifest.rs`: read all `.toml` files from manifests directory, deserialize via shared types, run validation, collect errors for invalid manifests and continue
- [x] T019 [US1] Add integration test `crates/compiler/tests/manifest_loading.rs`: load sample manifests directory, verify valid manifests parse (including driver manifest with `[hardware]` section), verify invalid manifest is skipped with error containing file path and field name

**Checkpoint**: Manifest format is defined and validated — US2 (compilation) can proceed

---

## Phase 4: User Story 2 — TOML to SQLite Compilation (Priority: P2)

**Goal**: Compiler produces `catalog.db` with 8 normalized tables and FTS5 search

**Independent Test**: Run compiler against manifests directory. Query `catalog.db` by ID and category.

- [x] T020 [US2] Implement SQLite schema creation in `crates/compiler/src/schema.rs`: create all 8 tables (packages, detection, install, checkver, hardware, backup, versions, meta) with indexes and FTS5 virtual table per data-model.md DDL
- [x] T021 [US2] Implement TOML → SQLite compilation in `crates/compiler/src/compile.rs`: iterate loaded manifests, insert into all relevant tables within a transaction. Write schema_version and compiled_at to meta table
- [x] T022 [US2] Implement version file aggregation in `crates/compiler/src/version_file.rs`: scan `versions/{id}/` directories, read JSON files, insert into versions table. Skip orphaned version dirs (no matching manifest)
- [x] T023 [US2] Wire up compiler main in `crates/compiler/src/main.rs`: load manifests → validate → create schema → compile → aggregate versions. Support --validate (dry-run, exit code 2 on errors) and --verbose
- [x] T024 [US2] Add integration test `crates/compiler/tests/compile.rs`: compile sample manifests to SQLite, query packages by ID, query by category, verify FTS5 search works for name/description/tags, verify version data is aggregated
- [x] T025 [P] [US2] Add integration test `crates/compiler/tests/validation_mode.rs`: run compiler with --validate against valid and invalid manifests, verify exit codes and error output

**Checkpoint**: Compiler produces queryable catalog.db from TOML manifests

---

## Phase 5: User Story 3 — Per-Version File Storage (Priority: P3)

**Goal**: Version checker writes individual JSON files per discovered version

**Independent Test**: Create a version file manually, run compiler, verify it appears in the database.

- [ ] T026 [US3] Implement version file writing in `crates/checker/src/version_writer.rs`: given a package ID, version string, URL, sha256, release_notes_url, pre_release flag — write to `versions/{id}/{version}.json`. Sanitize version string for filesystem safety
- [ ] T027 [US3] Add integration test `crates/checker/tests/version_writer.rs`: write a version file, read it back, verify fields. Test with semver, date, and unsafe characters in version string

**Checkpoint**: Version files can be written and read by the compiler

---

## Phase 6: User Story 4 — Automated Version Checking (Priority: P4)

**Goal**: Checker discovers new versions using 9 check methods with parallel execution, retry, and rate limiting

**Independent Test**: Run checker against a subset of manifests. Verify it discovers latest versions.

### Check Provider Infrastructure

- [ ] T028 [US4] Define `CheckProvider` trait in `crates/checker/src/providers/mod.rs`: async fn `check(manifest, client) -> Result<CheckResult>` where CheckResult contains version, url, sha256, release_notes_url, pre_release. Implement provider dispatch based on `checkver.provider` field
- [ ] T029 [US4] Implement hash discovery in `crates/checker/src/hash.rs`: given hash config from manifest, fetch hash via url+regex, jsonpath, or download-and-compute. One method per manifest based on which fields are present

### Individual Providers

- [ ] T030 [P] [US4] Implement `github` provider in `crates/checker/src/providers/github.rs`: query GitHub Releases API, filter pre-releases based on `include_pre_release`, extract version from tag name, get download URL and release notes URL
- [ ] T031 [P] [US4] Implement `gitlab` provider in `crates/checker/src/providers/gitlab.rs`: query GitLab Tags API, filter pre-releases based on `include_pre_release`, extract version from tag name
- [ ] T032 [P] [US4] Implement `direct_url` provider in `crates/checker/src/providers/direct_url.rs`: fetch URL, extract version from response body using regex
- [ ] T033 [P] [US4] Implement `http_head` provider in `crates/checker/src/providers/http_head.rs`: send HEAD request, extract version from Content-Disposition or Location headers using regex
- [ ] T034 [P] [US4] Implement `html_scrape` provider in `crates/checker/src/providers/html_scrape.rs`: fetch HTML page with reqwest, parse with scraper, extract version using regex
- [ ] T035 [P] [US4] Implement `browser_scrape` provider in `crates/checker/src/providers/browser_scrape.rs`: launch chromiumoxide browser, navigate to URL (60s timeout), wait for DOM (30s timeout), extract version using regex. Spawn handler on tokio
- [ ] T036 [P] [US4] Implement `pe_download` provider in `crates/checker/src/providers/pe_download.rs`: download executable with reqwest, parse PE headers with goblin, extract FileVersion field
- [ ] T037 [P] [US4] Implement `manual` provider in `crates/checker/src/providers/manual.rs`: skip check, log "manual: requires human update", return skip result

### Concurrency and Resilience

- [ ] T038 [US4] Implement rate limiting in `crates/checker/src/rate_limit.rs`: track per-provider pause windows from HTTP 429 and retry-after headers. Checker skips providers in their pause window
- [ ] T039 [US4] Wire up parallel checking in `crates/checker/src/main.rs`: load manifests, create reqwest client with retry middleware (3 attempts, 1s/2s/4s backoff), run checks with `futures::stream::buffer_unordered(concurrency)`, collect results. Support --filter and --concurrency flags
- [ ] T040 [US4] Implement checker state management in `crates/checker/src/state.rs`: load `checker-state.json`, update failure counts after each check, reset on success, write back after all checks complete

### Tests

- [ ] T041 [US4] Add integration test `crates/checker/tests/github_provider.rs`: test github provider against a known public repo (e.g., nightwatch-astro/astro-up-manifests) to verify version extraction
- [ ] T042 [P] [US4] Add integration test `crates/checker/tests/version_format.rs`: test version parsing and ordering for semver, date, and custom regex formats
- [ ] T042a [P] [US4] Add integration test `crates/checker/tests/template_e2e.rs`: end-to-end test — manifest with `$version` in `checkver.autoupdate` URL, checker discovers version, template variables resolve in the written version file URL

**Checkpoint**: Checker discovers versions for all provider types with bounded concurrency

---

## Phase 7: User Story 5 — Simplified CI Pipeline (Priority: P5)

**Goal**: Single GitHub Actions workflow: check → compile → sign → publish on 6-hour cron

**Independent Test**: Trigger workflow manually. Verify it checks, compiles, signs, and publishes.

- [ ] T043 [US5] Implement GitHub issue auto-create/close in `crates/checker/src/issue.rs`: after state update, for manifests with consecutive_failures >= 8 and no existing issue, create issue via GitHub REST API (reqwest, already a dependency). For manifests with issue_number and 0 failures, close issue via GitHub REST API. Update state with issue_number. Use `GITHUB_TOKEN` env var for authentication
- [ ] T044 [US5] Implement checker summary output in `crates/checker/src/main.rs`: print summary to stdout after all checks (new versions found, failures, skipped, persistent failures with issue numbers)
- [ ] T045 [US5] Create `.github/workflows/pipeline.yml`: single job on `schedule: cron '0 */6 * * *'` and `workflow_dispatch` with `filter` input (optional string, passed as `--filter` to checker — substring match on package ID, category, or provider). Steps: checkout, install Rust toolchain, cache Chromium binary, run checker (with filter if provided), run compiler, install minisign, sign catalog.db, create/upload to `catalog/latest` release via `gh release upload --clobber`, commit checker-state.json and new version files, push. Use concurrency group to prevent parallel runs
- [ ] T046 [US5] Add Chromium binary caching to CI: download Chromium on first run, cache via `actions/cache` keyed on Chromium version

**Checkpoint**: Full CI pipeline operational

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Logging, error messages, and final quality

- [ ] T047 [P] Configure tracing-subscriber in both `crates/compiler/src/main.rs` and `crates/checker/src/main.rs`: structured logging to stderr, respect --verbose flag for debug level
- [ ] T048 [P] Ensure all error messages include file path and field name for manifest parsing errors across compiler and checker
- [ ] T049 Port remaining astrophotography package data from old repo into `manifests/` TOML files (95 packages total, using old manifests as data reference)
- [ ] T050 Run full pipeline end-to-end: checker against all 95 manifests, compiler to catalog.db, sign with minisign, verify signature round-trips (sign then verify with public key), verify SC-002 (size <= 150KB), SC-003 (< 15 minutes)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — manifest format definition
- **US2 (Phase 4)**: Depends on US1 (needs manifests to compile)
- **US3 (Phase 5)**: Depends on Phase 2 only — version file writing is independent
- **US4 (Phase 6)**: Depends on US3 (needs version writer) + Phase 2 (shared types)
- **US5 (Phase 7)**: Depends on US2 + US4 (needs both compiler and checker working)
- **Polish (Phase 8)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: After Phase 2 — defines the manifest format
- **US2 (P2)**: After US1 — compiles manifests to SQLite
- **US3 (P3)**: After Phase 2 — independent of US1/US2
- **US4 (P4)**: After US3 — uses version writer, needs shared types
- **US5 (P5)**: After US2 + US4 — integrates compiler and checker into CI

### Within Each User Story

- Models/types before services
- Services before CLI integration
- Core implementation before tests
- Commit after each task or logical group

### Parallel Opportunities

- T001-T003: All Cargo.toml files can be created in parallel
- T005-T006: CLI skeletons in parallel
- T008-T012: All shared crate modules (after T007 manifest types)
- T014-T015: Shared crate tests in parallel
- T016-T017: Sample manifests in parallel
- T030-T037: All 8 check providers in parallel (after T028 trait)
- T041-T042: Checker tests in parallel
- T047-T048: Logging and error messages in parallel

---

## Implementation Strategy

### MVP First (US1 + US2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (shared crate)
3. Complete Phase 3: US1 (manifest format)
4. Complete Phase 4: US2 (compilation)
5. **STOP and VALIDATE**: Compile sample manifests, query catalog.db
6. This delivers a working compiler — the core artifact

### Incremental Delivery

1. Setup + Foundational → shared crate ready
2. US1 → manifest format defined, sample manifests → compiler can load them
3. US2 → compiler produces catalog.db → core artifact delivered
4. US3 → version file writing → checker can persist discoveries
5. US4 → automated version checking → full automation
6. US5 → CI pipeline → hands-free operation
7. Polish → logging, 95 manifests ported, end-to-end validation
