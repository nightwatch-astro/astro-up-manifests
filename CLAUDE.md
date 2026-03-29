# astro-up-manifests

Astrophotography software manifest repository — TOML definitions, Rust compiler (TOML → SQLite), and version checker.

## Structure

```
manifests/           # TOML manifest files per software package
versions/            # Per-version JSON files (discovered by checker)
crates/
  compiler/          # TOML → SQLite compiler
  checker/           # Version checker (GitHub, GitLab, HTML scrape)
```

## Commands

```sh
cargo run -p astro-up-compiler -- --manifests manifests --output catalog.db
cargo run -p astro-up-checker -- --manifests manifests --output versions
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Manifest Format

Each TOML file in `manifests/` defines a software package with sections: metadata, [detection], [install], [checkver], [dependencies], [hardware], [backup].

Consistent snake_case naming: `inno_setup` (not `innosetup`), `zip_wrap`, `download_only`, `pe_file`.

## Version Files

`versions/{package-id}/{semver}.json` — one file per discovered version:
```json
{"url": "...", "sha256": "...", "discovered_at": "...", "release_notes_url": "..."}
```

## Release

release-plz with nightwatch-astro shared workflow. Binaries are not published to crates.io.
