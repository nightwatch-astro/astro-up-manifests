# Quickstart: Manifest Pipeline

## Prerequisites

- Rust stable toolchain
- Chromium or Chrome (for `browser_scrape` — CI caches binary)
- `minisign` CLI (for signing in CI)

## Build

```sh
cargo build --workspace
```

## Validate manifests

```sh
cargo run -p astro-up-compiler -- --manifests manifests --validate
```

## Compile catalog

```sh
cargo run -p astro-up-compiler -- --manifests manifests --versions versions --output catalog.db
```

## Check versions

```sh
# All manifests, default concurrency (10)
cargo run -p astro-up-checker -- --manifests manifests --versions versions

# Filter by category
cargo run -p astro-up-checker -- --manifests manifests --versions versions --filter capture

# Single manifest
cargo run -p astro-up-checker -- --manifests manifests --versions versions --filter nina-app
```

## Write a manifest

Create `manifests/my-app.toml`:

```toml
manifest_version = 1
id = "my-app"
name = "My App"
description = "An astrophotography tool"
publisher = "Author Name"
homepage = "https://example.com"
category = "utility"
type = "application"
slug = "my-app"
tags = ["imaging", "utility"]

[install]
method = "inno_setup"
scope = "user"
elevation = false

[checkver]
provider = "github"
owner = "author"
repo = "my-app"
version_format = "semver"
include_pre_release = false

[detection]
method = "registry"
registry_key = "HKCU\\Software\\MyApp"
registry_value = "Version"
```

## CI pipeline

The pipeline runs on a 6-hour cron schedule:

1. Check versions: `astro-up-checker`
2. Compile catalog: `astro-up-compiler`
3. Sign: `minisign -Sm catalog.db -s $MINISIGN_KEY`
4. Publish: `gh release upload catalog/latest catalog.db catalog.db.minisig --clobber`
5. Commit state: `checker-state.json` + new version files
