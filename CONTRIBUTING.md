# Contributing

Keep changes focused. Include meaningful tests for behavior changes, update docs
when behavior or architecture changes, and preserve type safety and deterministic
output. Avoid unrelated refactoring or abstractions for hypothetical integrations.

## Setup and checks

Install Rust stable, Node 22.12+, and pnpm 10. Run `pnpm install --frozen-lockfile`
and `pnpm check`. `cargo build --workspace` and `cargo test --workspace` work
independently of the Node toolchain. Use `pnpm format` before submitting.

Astro templates are verified by `astro check`. Biome handles TypeScript, JSON,
CSS, and template frontmatter; Prettier with its Astro plugin formats only `.astro`
templates, which Biome cannot format in full. Biome's unused-variable rule is
disabled only for `.astro`, because it does not see variable use in templates;
Astro's checker retains that coverage. Keep templates readable and accessible.

Try changes against `examples/kitchen-sink` and the project's own `docs/`. Keep
the primary example healthy; failure cases belong in dedicated test fixtures.
No application/tooling logic may be authored in plain JavaScript. Generated
third-party JavaScript output is allowed.

Open a concise issue for substantial design changes. A PR should explain the
problem, resulting behavior, and checks run. Pre-stable does not mean low-quality.
