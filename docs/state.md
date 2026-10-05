# Current state

Version 0.2.0 implements a zero-config compiler for `docs/**/*.md` and a public
showcase built from its own output.

## Implemented commands

| Command | Result |
| --- | --- |
| `entwine check [project]` | Diagnostics and non-zero exit on errors |
| `entwine build [project]` | Static docs, copied assets, graph in `dist/` |
| `entwine graph [project]` | Identical to `build`; reports the graph page location |
| `entwine context [project]` | Complete deterministic Markdown context |
| `entwine context [project] --json` | Versioned structured JSON context |
| `entwine dev [project] --port 4173` | Watched build and loopback HTTP server |

## Supported content

CommonMark headings, paragraphs, emphasis, lists, links, code, blockquotes, GFM
tables, strikethrough, and task lists. Optional YAML `title`, `type`, and `status`
metadata are strings; unknown keys are ignored. Titles fall back to first H1,
then humanized filename. Repeated headings get suffixes starting at `-2`.

Document links, extensionless routes, anchors, and local assets are resolved.
Repeated links yield one document relationship. Orphans are warnings.
Projects without an index document receive an automatically generated landing page.

## Reading and graph views

Documentation uses neutral gray surfaces with automatic light/dark support.
Mobile navigation and page headings use native collapsible menus. The graph
centers the most connected document, distributes other documents in radial rings,
and draws directed curved edges between them. The overview fits the viewport;
an optional large view allows closer inspection with scrolling. Document and
relationship lists remain available as accessible collapsible sections.

## Public showcase and distribution

One Cloudflare Pages artifact serves the Astro website at `/`, Entwine's own
documentation at `/docs/`, and the kitchen-sink example at `/demo/`. The last two
are unmodified `entwine build` output hosted below a subpath. `pnpm build:showcase`
produces the same `deployment/` directory locally and verifies every relative
link, asset, and fragment under both mount points. Tagged `v*` releases publish
native binaries for Linux x86_64, macOS arm64, macOS x86_64, and Windows x86_64
on GitHub Releases, with SHA-256 checksums.

`build` and `graph` intentionally do the same work: the graph is always part of
a published site, and the graph page is only meaningful beside the pages it links.
`graph` remains as a convenience name; it is not a separate output mode.

## Portability checks

Routes that differ only by case are errors, because macOS and Windows file systems
would merge their output. Tests cover Unix and Windows separators, nested index
routes, traversal, the reserved `__entwine` path, deep trees, long filenames,
repeated and missing headings, README-style Markdown, large code blocks, and
graphs of 1, 5, 20, 50, and 100 nodes.

## Limits

No search, theme configuration, graph dragging or physics, incremental
compilation, or browser reload. Refresh after dev rebuilds. The graph uses a
fixed SVG layout with a complete text relationship list; at 100 nodes it is a wide
scrollable canvas, not a readable overview. Binaries are unsigned, and Linux builds
link the system glibc.
A page that links to its own heading counts as referencing itself. Raw HTML is escaped. Symbolic links are rejected. Builds own `dist/` entirely.
Context schema 0.1 is experimental. API compatibility is not guaranteed before 1.0.

Read the [architecture](architecture.md) and [future roadmap](roadmap.md).
