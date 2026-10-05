# Current state

Version 0.1.0 implements a zero-config compiler for `docs/**/*.md`.

## Implemented commands

| Command | Result |
| --- | --- |
| `entwine check [project]` | Diagnostics and non-zero exit on errors |
| `entwine build [project]` | Static docs, copied assets, graph in `dist/` |
| `entwine graph [project]` | Same full build, including linked SVG graph |
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

## Limits

No search, theme configuration, client-side graph interactions, incremental
compilation, or browser reload. Refresh after dev rebuilds. The graph uses a
fixed SVG layout with a complete text relationship list for dense repositories.
Raw HTML is escaped. Symbolic links are rejected. Builds own `dist/` entirely.
Context schema 0.1 is experimental. API compatibility is not guaranteed before 1.0.

Read the [architecture](architecture.md) and [future roadmap](roadmap.md).
