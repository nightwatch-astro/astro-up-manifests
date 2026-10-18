# astro-up-manifests

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

Astrophotography software manifest repository — TOML definitions, Rust compiler (TOML → SQLite), and version checker.

## Verifying released artifacts

Two independent checks cover the published artifacts, and they answer different
questions. Use whichever fits your situation — or both.

**Minisign signature (offline).** `catalog.db` ships with a `catalog.db.minisig`
signature made by the pipeline's key. Verify it against the public key that `astro-up`
itself embeds ([`minisign.pub`](https://github.com/nightwatch-astro/astro-up/blob/main/minisign.pub)
in the astro-up repo):

```bash
gh release download catalog/latest --pattern 'catalog.db*'
curl -sO https://raw.githubusercontent.com/nightwatch-astro/astro-up/main/minisign.pub
minisign -Vm catalog.db -p minisign.pub
```

This needs nothing but the files and the `minisign` binary, so it works air-gapped and
does not depend on GitHub being reachable.

> **Do not use the `minisign.pub` in this repository.** It is stale — key id
> `DAA8695754F367F7`, while the pipeline signs with `09B99E3253213132` — so verification
> against it fails with a key-id mismatch. Tracked separately; the file in `astro-up` is
> the authoritative copy.

**Build provenance attestation (online).** Every released binary and the catalog also
carry a GitHub artifact attestation: a signed statement recording which workflow run, at
which commit, produced those exact bytes.

```bash
# Tool binaries
gh attestation verify astro-up-compiler -R nightwatch-astro/astro-up-manifests
gh attestation verify astro-up-checker  -R nightwatch-astro/astro-up-manifests

# Catalog database
gh attestation verify catalog.db -R nightwatch-astro/astro-up-manifests
```

Unlike the minisign check, this one contacts GitHub — the attestation lives with the
repository, not in the release assets. Add `--format json` for the full predicate.

## License

This project is licensed under the GNU Affero General Public License v3.0 — see [LICENSE](LICENSE) for details.

If you modify this software and make it available over a network, you must make your modified source code available under the same license.
