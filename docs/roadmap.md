# Roadmap

This page describes future intent. The [current state](state.md) lists what works.

## Next

Continue collecting evidence from real adopters of the standalone compiler.

- Run the generated GitHub Pages, GitLab Pages, and Bitbucket pipelines in
  dedicated provider repositories. Local contract tests do not replace hosted
  provider execution.
- Measure dense relationship graphs and deeply nested trees beyond the current
  1,000-document baseline. Preserve complete static text views and simple full
  rebuilds unless evidence demonstrates a need for more complexity.
- Evaluate additional repository reference types (directories and images outside
  `docs/`) only when useful source-link behavior can be specified safely.
- Keep improving diagnostics from consumer reports and track context 0.4 adoption.
- Gather feedback on [agent knowledge](specs/agent-knowledge.md) and the
  interactive graph from real repositories: which conventions are missing, how
  scope should be presented, and where the graph needs more than filtering and
  focus. Instruction scope stays structural unless a clear, tool-neutral need
  appears.
- Persist user-chosen graph layouts and extend filtering only if real use shows
  the need; the text index remains authoritative.

The [hardening evidence](hardening.md) records completed compatibility fixes,
measurements, provider evidence, and remaining limits.

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
- Search and broader distribution, evaluated separately. Configuration is
  deliberately small (`entwine.toml`) and grows only with concrete needs. Preserve the
  [canonical model boundary](architecture.md#canonical-representation).

Entwine will not generate documentation with AI or invent project facts; it
scaffolds questions and leaves answers to people and their tools.
