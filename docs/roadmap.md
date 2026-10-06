# Roadmap

This page describes future intent. The [current state](state.md) lists what works.

## Next

Make 0.1 trustworthy in real repositories and publish it.

- Publish the first release so generated CI can install `@entwine/cli` from npm,
  then run the generated GitHub, GitLab, and Bitbucket pipelines on real
  repositories and fix what they reveal.
- Resolve the gaps found by running `entwine check` on ForgeCMS, Flowview, and
  Agentyx. Links from docs to repository files outside `docs/` (such as
  `../CLAUDE.md` or `../apps/...`), links to raw-HTML anchors
  (`<a id="...">`), and links to a directory without an index page are all
  errors today and are the most common blockers. A `docs/README.md` entry point
  is not yet recognized as the project role. Decide each deliberately; the
  regression tests in `crates/entwine-engine/tests/compilation.rs` pin current
  behavior.
- Improve graph readability beyond roughly 50 documents, where the fixed radial
  layout produces a wide canvas that the text relationship list serves better.

## Later

Directions that depend on evidence from real use. None is promised, and none
commits Entwine to a specific provider or integration.

- Documentation drift and stale-document hints.
- Change-aware recommendations after code changes.
- Better project-state assistance for people maintaining `state.md`.
- Optional integration with coding agents through structured context, which stays
  the stable boundary.
- Semantic context selection, so tools can request the relevant slice of
  knowledge.
- Additional knowledge roles, if the [convention](convention.md) proves too small.
- Search, small opt-in configuration, and broader distribution, evaluated
  separately. Preserve the
  [canonical model boundary](architecture.md#canonical-representation).

Entwine will not generate documentation with AI or invent project facts; it
scaffolds questions and leaves answers to people and their tools.
