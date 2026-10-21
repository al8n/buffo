# Releasing buffo

The `[package].version` in `Cargo.toml` is the release source of truth. Release
tags must be exactly `v<version>`; for example, version `0.9.0` uses tag
`v0.9.0`. The crates.io workflow rejects a tag that does not match the Cargo
package version.

## Before tagging

From a clean checkout of the release commit, confirm the changelog describes
the release and run:

```bash
cargo test --all-features
cargo package
cargo publish --dry-run
```

Review the package contents reported by Cargo. Do not create the tag until all
checks pass and the intended tag is exactly `v` followed by the version in
`Cargo.toml`.

## Trusted publishing

crates.io publication uses GitHub Actions OIDC. The publish job runs in the
protected `crates-io` GitHub environment, which requires approval from `al8n`,
and obtains a short-lived token from `rust-lang/crates-io-auth-action@v1`.

The crates.io trusted publisher must match these values:

- GitHub owner: `al8n`
- Repository: `buffo`
- Workflow: `.github/workflows/crates.yml`
- Environment: `crates-io`

Pushing a matching `v*` tag is the only event that can publish. Pull requests
and manual `workflow_dispatch` runs execute verification only.

## After tagging

Approve the protected environment only after reviewing the tag workflow and
its verified package. Once the workflow succeeds, confirm crates.io and docs.rs
show the expected version, inspect the published metadata, and test `buffo` in
a fresh downstream Cargo project. Investigate a failed workflow rather than
moving or replacing an existing release tag.
