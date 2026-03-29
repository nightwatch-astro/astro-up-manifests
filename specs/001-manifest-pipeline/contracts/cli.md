# CLI Contracts

## astro-up-compiler

```
astro-up-compiler [OPTIONS]

Options:
  --manifests <DIR>     Path to manifests directory [default: manifests]
  --versions <DIR>      Path to versions directory [default: versions]
  --output <FILE>       Output SQLite database path [default: catalog.db]
  --validate            Dry-run: validate manifests only, no output
  --verbose             Enable verbose logging
  --help                Print help
  --version             Print version

Exit codes:
  0    Success (or validation passed)
  1    Fatal error (cannot read manifests dir, cannot write output)
  2    Validation errors found (--validate mode only)
```

### Output artifacts

| File | Description |
|------|-------------|
| `catalog.db` | Compiled SQLite database |
| stdout | Progress and summary (human-readable) |
| stderr | Errors and warnings (structured with tracing) |

---

## astro-up-checker

```
astro-up-checker [OPTIONS]

Options:
  --manifests <DIR>     Path to manifests directory [default: manifests]
  --versions <DIR>      Path to versions directory [default: versions]
  --state <FILE>        Path to state file [default: checker-state.json]
  --concurrency <N>     Max concurrent checks [default: 10]
  --filter <PATTERN>    Only check manifests matching pattern (id, category, or provider)
  --verbose             Enable verbose logging
  --help                Print help
  --version             Print version

Exit codes:
  0    Success (all checks completed, some may have failed gracefully)
  1    Fatal error (cannot read manifests dir, invalid state file)
```

### Output artifacts

| File | Description |
|------|-------------|
| `versions/{id}/{ver}.json` | New version files (one per discovered version) |
| `checker-state.json` | Updated failure counts and timestamps |
| stdout | Progress and summary (human-readable) |
| stderr | Errors and warnings (structured with tracing) |

### Summary output format (stdout)

```
Checked 95 manifests (10 concurrent)
  New versions: 3 (nina-app 3.1.2, phd2 2.6.13, stellarium 24.4)
  Failed: 2 (sharpcap, astap)
  Skipped: 5 (manual)
  Persistent failures: 1 (sharpcap — 8 consecutive, issue #42 created)
```
