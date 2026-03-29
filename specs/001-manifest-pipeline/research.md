# Research: Manifest Pipeline

## Crate Decisions

### chromiumoxide — INCLUDE

- **Decision**: Include `chromiumoxide` in the checker crate for `browser_scrape` provider
- **Rationale**: Existing manifests use Rod-based browser scraping (Go). These packages need a working `browser_scrape` provider from day one. `chromiumoxide` is the most capable async CDP client in Rust, tokio-native.
- **Key patterns**: Handler MUST be spawned on tokio or browser deadlocks. Per-page timeouts via `BrowserConfig::builder().request_timeout()`. Requires Chromium binary — CI caches it.
- **Alternatives considered**: Shelling out to a browser CLI (fragile), `headless_chrome` (sync-only, incompatible with async checker)
- **Dependency isolation**: `chromiumoxide` is only a dependency of the checker crate, not the compiler. Heavy dependency stays contained.

### garde / validator — SKIP

- **Decision**: Custom validation after serde deserialization
- **Rationale**: `serde` catches structural issues (missing fields, wrong types). Semantic validation (e.g., "if install method is inno_setup, certain fields must be present") is better expressed as explicit Rust functions than derive macros. Fewer dependencies, simpler code.
- **Alternatives considered**: `garde` (cleaner API, smaller community), `validator` (larger ecosystem). Both add derive macro complexity for validation rules that are clearer as functions.

### minisign — CLI in CI

- **Decision**: Sign `catalog.db` via `minisign` CLI in the CI pipeline, not in Rust code
- **Rationale**: Signing is a CI-only operation. Shelling out avoids handling the secret key in Rust code. The `minisign` binary is small and easy to install in CI. The `minisign-verify` crate (already in workspace) handles client-side verification.
- **Alternatives considered**: `minisign` Rust crate (0.7.x) — works but adds unnecessary complexity for a CI script task.

### lenient_semver — ADD

- **Decision**: Add `lenient_semver` (~0.4) for parsing non-strict version strings
- **Rationale**: Astrophotography software uses varied version formats ("2024.1", "3.2", "v1.2.3"). `semver` v1 is strict and rejects these. `lenient_semver` parses into `semver::Version` with missing parts filled as `.0`.
- **Used when**: `version_format = "semver"` (default). Custom regex and date formats use separate parsing.

### reqwest-middleware + reqwest-retry — ADD

- **Decision**: Add `reqwest-middleware` + `reqwest-retry` for HTTP resilience
- **Rationale**: The checker makes hundreds of HTTP requests per run. Transient failures (DNS, timeouts, 5xx) are common. Middleware-based retry with exponential backoff is cleaner than manual retry loops.
- **Configuration**: 3 attempts, exponential backoff (1s/2s/4s), retry on transient errors only.

### scraper — ADD

- **Decision**: Add `scraper` for HTML parsing in `html_scrape` provider
- **Rationale**: Lightweight HTML/CSS selector engine. Combined with `reqwest` for fetching, replaces the need for a full browser for static HTML scraping.

### futures — USE buffer_unordered

- **Decision**: Use `futures::stream::StreamExt::buffer_unordered` for bounded concurrency
- **Rationale**: Cleaner than manual `Semaphore` for "check N manifests with max M concurrent". Maps directly to `--concurrency` CLI flag.

## Architecture Decisions

### Compiler is synchronous

The compiler reads TOML files from disk and writes SQLite. No network I/O, no async needed. This keeps the compiler simple and avoids `rusqlite` + async complications (`Connection` is `!Send`).

### Checker is async

The checker makes HTTP requests in parallel. `tokio` runtime with `reqwest` client. SQLite writes (if any) go through `tokio::task::spawn_blocking`.

### State file for failure tracking

`checker-state.json` in repo root tracks consecutive failure counts per manifest. Updated each pipeline run, committed alongside version files. Schema:
```json
{
  "nina-app": { "consecutive_failures": 0, "last_checked": "2026-03-29T12:00:00Z" },
  "phd2": { "consecutive_failures": 3, "last_checked": "2026-03-29T12:00:00Z" }
}
```

### Version ordering strategy

- `semver` format: parsed with `lenient_semver`, ordered by semver rules
- `date` format: parsed as `NaiveDate` (chrono or time crate), ordered chronologically
- Custom regex: capture groups provide ordering components, ordered lexicographically per group
- Fallback: if parsing fails, order by `discovered_at` timestamp

### Dependency summary

| Crate | Version | Compiler | Checker | Purpose |
|-------|---------|----------|---------|---------|
| serde + serde_json + toml | latest | Yes | Yes | Serialization |
| rusqlite (bundled) | 0.32 | Yes | No | SQLite compilation |
| semver + lenient_semver | 1.x / 0.4 | Yes | Yes | Version parsing |
| reqwest (json) | 0.12 | No | Yes | HTTP client |
| reqwest-middleware + reqwest-retry | latest | No | Yes | Retry resilience |
| tokio (full) | 1.x | No | Yes | Async runtime |
| futures | latest | No | Yes | buffer_unordered |
| scraper | latest | No | Yes | HTML parsing |
| regex | latest | No | Yes | Version extraction |
| clap (derive) | latest | Yes | Yes | CLI argument parsing |
| sha2 | latest | No | Yes | SHA256 computation |
| chrono | latest | Yes | Yes | Timestamps |
| tracing + tracing-subscriber | latest | Yes | Yes | Structured logging |
| chromiumoxide | ~0.7 | No | Yes | Headless browser for browser_scrape |
| goblin | latest | No | Yes | PE header parsing |
