# Contributing

Keep changes focused. Include meaningful tests for behavior changes, update docs
when behavior or architecture changes, and preserve type safety and deterministic
output. Avoid unrelated refactoring or abstractions for hypothetical integrations.

## Setup and checks

Install Rust stable, Node 22.18+ (repository tooling runs TypeScript directly), and pnpm 10. Run `pnpm install --frozen-lockfile`
and `pnpm check`. `cargo build --workspace` and `cargo test --workspace` work
independently of the Node toolchain. Use `pnpm format` before submitting.

Astro templates are verified by `astro check`. Biome handles TypeScript, JSON,
CSS, and template frontmatter; Prettier with its Astro plugin formats only `.astro`
templates, which Biome cannot format in full. Biome's unused-variable rule is
disabled only for `.astro`, because it does not see variable use in templates;
Astro's checker retains that coverage. Keep templates readable and accessible.

Try changes against `examples/kitchen-sink` and the project's own `docs/`. Keep
the primary example healthy; failure cases belong in dedicated test fixtures.
No application/tooling logic may be authored in plain JavaScript; repository
tooling lives in strict TypeScript under `tooling/`. Generated third-party
JavaScript output is allowed. Consumer projects remain stack-agnostic.

`pnpm build:showcase` builds the Astro site, Entwine's own docs, and the
kitchen-sink, composes `deployment/`, and verifies subpath hosting.

## Releasing

1. Bump the workspace version in `Cargo.toml`, the crate dependency versions,
   and both `package.json` files; update `docs/state.md` and `docs/roadmap.md`.
2. Merge to `main` once CI passes.
3. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.

The `Release` workflow refuses a tag that differs from the `entwine-cli` version,
runs the full CI workflow, builds Linux x86_64, macOS arm64/x86_64, and Windows
x86_64 archives, checks them against `SHA256SUMS`, and creates a pre-release on
GitHub. Releases are never created from branch pushes.

Open a concise issue for substantial design changes. A PR should explain the
problem, resulting behavior, and checks run. Pre-stable does not mean low-quality.
