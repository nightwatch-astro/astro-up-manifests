<!--
Sync Impact Report
- Version change: 0.0.0 → 1.0.0
- Added principles: Data Contract First, Compile-Time Safety, Graceful Degradation, Manifest Compatibility
- Added sections: Technical Constraints, Development Workflow
- Templates requiring updates:
  - .specify/templates/plan-template.md ✅ no changes needed (Constitution Check is generic)
  - .specify/templates/spec-template.md ✅ no changes needed (generic template)
  - .specify/templates/tasks-template.md ✅ no changes needed (generic template)
  - .specify/templates/commands/ — no command files exist
- Follow-up TODOs: none
-->

# astro-up-manifests Constitution

## Core Principles

### I. Data Contract First

The TOML manifest format is the foundational data contract. Every downstream
consumer (compiler, checker, client app, CI pipeline) depends on it.

- Manifest schema changes MUST be versioned via `manifest_version`
- Breaking schema changes MUST increment `manifest_version` and provide migration
- Field naming MUST use consistent `snake_case` (`inno_setup`, `zip_wrap`,
  `pe_file`, `download_only`)
- The compiled SQLite schema (`catalog.db`) is versioned independently from
  manifests — breaking DB changes require a `schema_version` bump in the `meta`
  table

### II. Compile-Time Safety

Correctness is enforced at compilation time via typed Rust structs, not at
runtime via ad-hoc validation.

- Manifest parsing MUST use `serde` deserialization into typed structs
- Field validation MUST use `garde` (or equivalent) with declarative constraints
- Invalid manifests MUST be skipped with a clear error (file path + field name),
  NEVER abort the entire compilation
- The compiler `--validate` flag MUST enable dry-run validation for CI PR checks

### III. Graceful Degradation

The pipeline MUST continue operating when individual components fail. A single
broken manifest or unreachable vendor MUST NOT block the entire pipeline.

- Compiler: skip invalid manifests, report errors, continue
- Checker: log failed version checks, continue to next manifest
- CI: no pipeline abort on transient failures — retry with exponential backoff
- Persistent failures MUST auto-create GitHub issues; resolved failures
  MUST auto-close them

### IV. Manifest Compatibility

Version discovery MUST match or exceed the accuracy of the existing Go checker.

- The Go client was never deployed to production, so no legacy `manifests.json`
  output is required. The compiler produces `catalog.db` only.
- If a legacy consumer is added in the future, a compatibility layer MUST be
  introduced via a separate subcommand or post-processing step — not by
  changing the primary output format

## Technical Constraints

- **Language**: Rust (stable toolchain), Cargo workspace with independent crates
- **Dependencies**: Minimize — prefer stdlib and well-maintained crates.
  `chromiumoxide` for browser scraping stays isolated in the checker crate
- **Naming**: `snake_case` everywhere — Rust code, TOML fields, JSON keys,
  CLI flags, file names
- **Concurrency**: Async with `tokio`. Checker runs parallel version checks
  (configurable concurrency, default 10)
- **CI**: Single-job pipeline — check → compile → sign → publish. Must complete
  in under 15 minutes for 95+ manifests
- **Distribution**: GitHub Releases rolling `catalog/latest` tag, minisign
  signatures. No crates.io publishing
- **Licensing**: Apache-2.0

## Development Workflow

- **Testing**: Integration tests preferred over mocks. Test against real TOML
  files and real SQLite databases. Unit tests for parsing logic and version
  comparison
- **Validation**: `cargo clippy -- -D warnings` and `cargo fmt --check` MUST
  pass in CI. No suppressed warnings without justification
- **Commits**: Atomic commits per logical change. Feature branches merged
  with `--no-ff`
- **Releases**: Automated via release-plz. Binary artifacts only (no library
  publishing)

## Governance

This constitution governs all development in the `astro-up-manifests` repository.

- **Amendments**: Require documentation of the change, rationale, and impact
  assessment on existing manifests and downstream consumers
- **Versioning**: Constitution follows semver — MAJOR for principle
  removals/redefinitions, MINOR for additions, PATCH for clarifications
- **Compliance**: All PRs MUST be checked against these principles. The plan
  template's Constitution Check gate enforces this

**Version**: 1.0.0 | **Ratified**: 2026-03-29 | **Last Amended**: 2026-03-29
