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

Try `entwine init` and `entwine setup --dry-run` in a scratch Git repository, or run
`pnpm smoke:setup`, which does so for every provider with the real binary. Generated CI
files have golden snapshots in `crates/entwine-cli/tests/golden/`; refresh them with
`ENTWINE_UPDATE_GOLDEN=1 cargo test -p entwine-cli --test convention` after reviewing
the diff.

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
   provenance (platform packages first, launcher last), waits for the registry to
   list every package at the exact version, installs from a fresh cache, runs a
   consumer smoke, and finally un-drafts the release (which creates the tag).

**A release PR is a release action, not a routine update.** Merge it only when you
intend to ship that version. Automation stays safe even if publication fails
afterwards, because of the pending-release guard:

- A draft release has no Git tag yet, so release-please would see all shipped
  commits as unreleased and open another (larger) release PR. While any
  `vX.Y.Z` draft exists, the `guard` job skips release-please: no release PR is
  created or updated, and the run stays green with a notice
  (`Release v0.5.0 is still pending ... Release → Run workflow → v0.5.0`).
- To resume a failed draft: Actions → Release → Run workflow → enter the tag. The
  run checks out the commit the draft targets, skips builds when every package and
  asset already exist, never republishes an npm version that is already public,
  re-runs the registry verification, and only then publishes the release.
- Registry lag is not a failure: verification polls the registry's metadata for the
  exact version with bounded backoff (20 minutes) and then installs with an
  isolated, online cache. A timeout leaves the draft intact and says whether the
  registry answered "not found" (not published or not propagated) or could not be
  reached; an install that works but a broken CLI is reported separately.
- More than one draft is an invalid state: the guard lists them all and does not
  guess; resume the one that matches `main` and delete the superseded drafts.
- npm versions are immutable. Never unpublish; fix forward with a new patch.

Required repository configuration: secret `NPM_TOKEN` (publish rights on the
`@entwine` npm scope) and Actions setting "Allow GitHub Actions to create and
approve pull requests". The workflow never runs from branches other than `main`.
CI on the release PR itself does not start automatically, because PRs created
with `GITHUB_TOKEN` do not trigger workflows; the full CI runs again before
anything is published.

`pnpm test:tooling` runs the release-logic tests (no network). `pnpm smoke:npm` reproduces the npm distribution locally: it stages and packs the
launcher plus this platform's binary package, installs both into a clean project,
and runs `entwine`.
