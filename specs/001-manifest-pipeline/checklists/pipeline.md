# Pipeline Requirements Quality Checklist: Manifest Pipeline Modernization

**Purpose**: Validate requirement completeness, clarity, and consistency across all pipeline domains with operational risk emphasis
**Created**: 2026-03-29
**Feature**: [spec.md](../spec.md)

## Requirement Completeness

- [ ] CHK001 - Are all 8 SQLite table schemas fully specified with column types and constraints, or only column names? [Completeness, Spec §FR-006]
- [ ] CHK002 - Are the default installer switches exhaustively listed for all supported installer types, or only InnoSetup and MSI? [Completeness, Spec §FR-005]
- [ ] CHK003 - Are requirements defined for what the `manual` check method does at runtime (skip, log, flag)? [Gap, Spec §FR-015]
- [ ] CHK004 - Are requirements specified for how the checker determines "latest" version when multiple releases exist (e.g., GitHub pre-release vs stable vs draft)? [Gap]
- [ ] CHK005 - Is the `pe_download` check method behavior specified (what is downloaded, how PE headers are inspected, which version field is extracted)? [Completeness, Spec §FR-015]
- [ ] CHK006 - Are requirements defined for the `direct_url` and `http_head` check methods (what response data is parsed, how version is extracted)? [Completeness, Spec §FR-015]
- [ ] CHK007 - Is the full set of manifest TOML sections documented with required vs optional fields per section? [Completeness, Spec §FR-001]
- [ ] CHK008 - Are requirements specified for how the FTS5 virtual table handles multi-word queries and ranking? [Gap, Spec §FR-006]

## Requirement Clarity

- [ ] CHK009 - Is "clobber mode" for release asset upload defined with specific behavior (delete then upload, or overwrite in place)? [Clarity, Spec §FR-009]
- [ ] CHK010 - Is "optional vendor/category filter" for manual dispatch specified with exact filter syntax and matching behavior? [Clarity, Spec §FR-010]
- [ ] CHK011 - Is the `manifest_version` field's role clearly defined — does bumping it trigger migration logic, or is it informational only? [Clarity, Spec §FR-001]
- [ ] CHK012 - Are the pre-release detection patterns explicitly enumerated beyond `-rc`, `-beta`, `-alpha` (e.g., `-dev`, `-snapshot`, `-nightly`)? [Clarity, Spec §FR-008]
- [ ] CHK013 - Is "clear error (file path + field name)" quantified — does it include the invalid value, expected type, and suggestion? [Clarity, Spec §FR-017]

## Operational Risk & Failure Handling

- [ ] CHK014 - Are requirements defined for how the checker persists failure count across pipeline runs (file, release metadata, issue label)? [Gap, Spec §FR-011]
- [ ] CHK015 - Are requirements specified for what happens when the minisign private key is missing or invalid in CI? [Gap, Spec §FR-007]
- [ ] CHK016 - Is the GitHub App token refresh mechanism specified (JWT refresh, installation token TTL, retry window)? [Completeness, Edge Case §6]
- [ ] CHK017 - Are requirements defined for CI runner resource constraints (disk space for browser binary, memory for 200+ manifest compilation)? [Gap, Edge Case §5]
- [ ] CHK018 - Is the concurrency group scope specified (per-repo, per-workflow, per-branch)? [Clarity, Edge Case §7]
- [ ] CHK019 - Are requirements defined for pipeline behavior when GitHub Releases API is unavailable during publish step? [Gap, Spec §FR-009]
- [ ] CHK020 - Are requirements specified for what happens when `versions/` directory has orphaned version files (manifest deleted but version files remain)? [Gap]
- [ ] CHK021 - Are auto-close requirements for resolved issues specified with a trigger condition (single success after 8 failures, or N consecutive successes)? [Clarity, Spec §FR-011]

## Data Contract & Migration

- [ ] CHK022 - Are requirements defined for validating migrated manifests against the new schema (acceptance criteria for "without data loss" in SC-001)? [Measurability, Spec §SC-001]
- [ ] CHK023 - Is the `[remote]` → `[checkver]` rename specified with field-level mapping for all subfields? [Completeness, Spec §FR-002]
- [ ] CHK024 - Are requirements defined for handling manifests that exist in the old repo but are invalid under the new schema? [Gap, Spec §SC-001]
- [x] CHK025 - N/A: Legacy `manifests.json` not required — Go client never deployed to production. Constitution Principle IV amended.
- [x] CHK026 - N/A: No transition period — greenfield deployment with `catalog.db` only.

## Acceptance Criteria Quality

- [x] CHK027 - SC-002 threshold updated to 200 KB to account for FTS5 index overhead. Measured at 156 KB with 96 manifests. [Measurability, Spec §SC-002]
- [ ] CHK028 - Is SC-004 (ETag conditional request) a requirement for the pipeline or the client? The pipeline publishes to GitHub Releases which provides ETag natively — is this a no-op? [Ambiguity, Spec §SC-004]
- [x] CHK029 - N/A: SC-006 does not exist in spec. Version discovery accuracy is covered by provider-specific integration tests and FR-015/FR-016.
- [ ] CHK030 - Are acceptance scenarios defined for the hash discovery tiers — what happens when tier 1 fails but tier 2 succeeds? [Coverage, Spec §FR-004]

## Scenario Coverage

- [ ] CHK031 - Are requirements defined for first-run behavior when `versions/` directory is empty (no prior version history)? [Coverage, Gap]
- [ ] CHK032 - Are requirements specified for manifest TOML files with valid syntax but semantically invalid data (e.g., `checkver.provider` set to an unknown method)? [Coverage, Edge Case]
- [ ] CHK033 - Are requirements defined for version strings that don't follow semver (e.g., date-based versions like `2026.03.29`, or single integers)? [Coverage, Gap]
- [ ] CHK034 - Are requirements specified for browser_scrape timeout and resource limits (page load timeout, max DOM wait, Chromium crash recovery)? [Gap, Spec §FR-015]
- [ ] CHK035 - Are requirements defined for what happens when a hash.url returns a page with multiple SHA256 hashes and the regex matches more than one? [Coverage, Spec §FR-004]

## Notes

- Focus: balanced across all domains, operational risk emphasis
- Depth: standard
- Audience: reviewer (spec quality gate before planning)
- Migration requirements included due to operational risk alignment
- 35 items across 6 categories
