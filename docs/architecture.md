# Architecture

```text
docs/*.md ─────────────┐
                       ├→ Parser → KnowledgeBase ─┬→ Context (complete)
AGENTS.md, SKILL.md … ─┘  (opt-in discovery)      ├→ Site / Knowledge ┐ publication
                                                  └→ Graph / Agents  ┘ policy applies
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

## Artifact kinds, configuration, and publication

Every `Document` has a [role](convention.md#roles-and-how-they-are-found) (what
it means) and an `ArtifactKind` (what kind of file it is: documentation, agent
instructions, or skill). Agent artifacts carry `AgentDetails` (convention, path,
scope, skill folder, name, description, resources). Provider file names are data
in `AgentConvention`; no code branches on a brand. Ids of agent artifacts start
with `repo:` so they can never collide with `docs/` paths.

`entwine.toml` is parsed into `Config` (core type, parsed in the engine) and is
optional; all defaults are the zero-config behavior. Discovery (`discovery.rs`)
only reads files and feeds the same parser. **Publication is a projection-time
decision:** the canonical `KnowledgeBase` and `ContextModel` always contain
discovered artifacts, while `project()` builds the `SiteModel`, `GraphModel`,
and agent views from a documentation-only subset unless
`include_agent_knowledge` is set. The unpublished HTML for a documentation link
to an agent file is rendered as a repository reference at resolution time, so no
output file mentions it.

## Graph

`graph_layout.rs` computes a deterministic force-directed layout using only
arithmetic and `sqrt` (no trigonometry, so coordinates match across platforms).
`graph_page.rs` renders the static SVG, legend, embedded JSON, and the full text
index. `graph.js` is a dependency-free enhancement; it reads the JSON and mutates
the existing SVG. SVG was kept over canvas for accessibility, a no-script
fallback, tiny output, and sufficient speed at 250 nodes (see
[hardening](hardening.md#graph-performance)).

The renderer is not the knowledge model. The built-in renderer consumes a
`SiteModel`, never source files. The official Astro website is a separate
consumer of product copy and does not render users' documentation.

The page renderer can be selected with `[site] renderer = "flowview"` in
`entwine.toml`. This is an experimental, opt-in comparison; omitting it keeps
the built-in renderer. Flowview is a dependency of `entwine-engine` only and
receives a narrow, host-prepared page view. Entwine retains route calculation,
the current raw-HTML policy, CSS, and generated Graph, Knowledge, and Agents
views. Its embedded template compiles once per site render. The current pinned
Flowview Git revision is temporary because the required crate was not found in
the crates.io index. See the [experiment record](decisions/flowview-renderer-experiment.md).

## Determinism and safety

Paths, routes, relations, assets, roles, and diagnostics are explicitly sorted.
Headings get stable unique IDs. SVG positions depend only on the sorted graph. Links are relative for subpath hosting.
Symbolic links and escaping paths are rejected, including by `init` and `setup`,
which never write through a symlink and never overwrite existing files.
Raw HTML is displayed as text except narrowly recognized empty anchors; unsafe
URL schemes are errors. Repository references never become knowledge relations.

The CLI stages a complete tree before replacing owned `dist/` output. Validation
errors preserve the last successful build. Unrelated `dist/` directories are
never replaced automatically.

See [current state](state.md) and [roadmap](roadmap.md) for the product boundary,
and [deployment](deployment.md) for how the portable `dist/` output is published.

## Repository sources

The [workspace manifest](../Cargo.toml) defines the implementation crates.
[Project instructions](../AGENTS.md) describe repository development rules.
Repository files remain references, distinct from Markdown knowledge relationships.
