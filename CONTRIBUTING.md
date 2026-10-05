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

Releases are automated; nobody tags or edits versions by hand.

1. Use [Conventional Commits](https://www.conventionalcommits.org). PRs are
   squash-merged and the **PR title** becomes the commit, so it must look like
   `feat: ...`, `fix: ...`, `perf: ...`, `docs: ...` (checked by the `PR title`
   workflow). While pre-1.0, `feat` bumps the minor version and `fix` the patch;
   `feat!:` or a `BREAKING CHANGE:` footer is a breaking change.
2. On every push to `main`, release-please opens or updates a **release PR** that
   bumps the version in `Cargo.toml`, `Cargo.lock`, and every `package.json` and
   writes `CHANGELOG.md`.
3. Merging the release PR creates a draft GitHub release. The `Release` workflow
   then runs the full CI, builds Linux x86_64, macOS arm64/x86_64, and Windows
   x86_64 binaries, uploads archives with `SHA256SUMS`, publishes
   `@entwine/cli` and its `@entwine/cli-<platform>` packages to npm with
   provenance (platform packages first, launcher last), and finally un-drafts
   the release. If any step fails, the release stays a draft and nothing partial
   is announced; re-run the failed jobs.

Required repository configuration: secret `NPM_TOKEN` (publish rights on the
`@entwine` npm scope) and Actions setting "Allow GitHub Actions to create and
approve pull requests". The workflow never runs from branches other than `main`.
CI on the release PR itself does not start automatically, because PRs created
with `GITHUB_TOKEN` do not trigger workflows; the full CI runs again before
anything is published.

`pnpm smoke:npm` reproduces the npm distribution locally: it stages and packs the
launcher plus this platform's binary package, installs both into a clean project,
and runs `entwine`.
