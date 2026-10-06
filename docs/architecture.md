# Architecture

```text
Markdown → Scanner → Parser → KnowledgeBase
                                  / | \
                         Site  Graph  Context  Knowledge overview
```

## Crate boundaries

`entwine-core` owns pure, serializable domain types, including the knowledge
role model and its classification rules. It performs no I/O.
`entwine-engine` depends on core and owns scanning, parsing, resolution,
validation, projections, and rendering. `entwine-cli` depends on the engine and
owns arguments, output publication, diagnostic presentation, HTTP serving,
file watching, `init` scaffolding, and `setup` (Git remote inspection and CI
templates). Provider detection and CI generation deliberately live in the CLI,
never in core or engine. See [the decision on publishing](decisions/provider-owned-publishing.md).

## Canonical representation

Each Markdown document is parsed into events once. The engine extracts metadata,
headings, and link positions during that pass, then derives the document's
[knowledge role](convention.md#roles-and-how-they-are-found) from its
frontmatter `type` and canonical path. After all routes are known it resolves
URLs in those events and renders the body without parsing again. Unique directed
relations are derived from resolved links. Backlinks, navigation, graph nodes,
the Knowledge overview, and the [context](specs/context-schema.md) all use that
same knowledge model.

The renderer is not the knowledge model. The built-in renderer consumes a
`SiteModel`, never source files. The official Astro website is a separate
consumer of product copy and does not render users' documentation.

## Determinism and safety

Paths, routes, relations, assets, roles, and diagnostics are explicitly sorted.
Headings get stable unique IDs. SVG positions depend only on sorted node order
(convention roles first, then path). Links are relative for subpath hosting.
Symbolic links and escaping paths are rejected, including by `init` and `setup`,
which never write through a symlink and never overwrite existing files.
Raw HTML is displayed as text and unsafe URL schemes are errors.

The CLI stages a complete tree before replacing owned `dist/` output. Validation
errors preserve the last successful build. Unrelated `dist/` directories are
never replaced automatically.

See [current state](state.md) and [roadmap](roadmap.md) for the product boundary,
and [deployment](deployment.md) for how the portable `dist/` output is published.
