# Architecture

```text
Markdown → Scanner → Parser → KnowledgeBase
                                  / | \
                               Site Graph Context
```

## Crate boundaries

`entwine-core` owns pure, serializable domain types. `entwine-engine` depends on
core and owns scanning, parsing, resolution, validation, projections, and rendering.
`entwine-cli` depends on the engine and owns arguments, output publication,
diagnostic presentation, HTTP serving, and file watching.

## Canonical representation

Each Markdown document is parsed into events once. The engine extracts metadata,
headings, and link positions during that pass. After all routes are known it
resolves URLs in those events and renders the body without parsing again.
Unique directed relations are derived from resolved links. Backlinks, navigation,
graph nodes, and context use that same knowledge model.

The renderer is not the knowledge model. The built-in renderer consumes a
`SiteModel`, never source files. The official Astro website is a separate consumer
of product copy and does not render users' documentation.

## Determinism and safety

Paths, routes, relations, assets, and diagnostics are explicitly sorted. Headings
get stable unique IDs. SVG positions depend only on sorted node order. Links are
relative for subpath hosting. Symbolic links and escaping paths are rejected.
Raw HTML is displayed as text and unsafe URL schemes are errors.

The CLI stages a complete tree before replacing owned `dist/` output. Validation
errors preserve the last successful build. Unrelated `dist/` directories are
never replaced automatically.

See [current state](state.md) and [roadmap](roadmap.md) for the product boundary.
