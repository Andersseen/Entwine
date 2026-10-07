---
name: cargo-workspace-verification
description: Run the Rust handoff gate and keep Cargo workspace metadata, lockfile and versions consistent.
---

# Cargo workspace verification

Use before declaring Rust work done, and when touching `Cargo.toml`, `Cargo.lock`, features or public crate APIs.

## The gate (all must pass)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

Fix clippy findings rather than adding `#[allow]`; if an allow is justified, scope it to the item with a comment saying why. `--locked` fails if `Cargo.lock` would change: commit lock updates deliberately.

## Workspace rules

- Shared dependencies and versions are owned by `[workspace.dependencies]` / `[workspace.package]`; member crates use `dep.workspace = true`. Do not pin a version in a member that the workspace already declares.
- Add a dependency only with a reason; prefer already-present crates. Check `default-features` and the feature set it enables.
- MSRV: `rust-version` in `[workspace.package]` is a contract. Avoid APIs newer than it; do not bump it casually.
- Features must be additive. If you add a feature, build with `--no-default-features` and `--all-features`.
- Public API changes (`pub` items, CLI flags, output formats, JSON schemas) need docs and tests; flag them in the summary.

## Release versions

The version lives in several files (`Cargo.toml`, `Cargo.lock`, `package.json`s). Never edit them by hand for a release; release-please owns them. `node tooling/check-versions.ts` verifies they agree.

## Verify

Report the exact commands run and their outcome. If you could not run one, say which and why. For this repository the full gate is `pnpm check` plus the CI jobs in `.github/workflows/ci.yml`.
