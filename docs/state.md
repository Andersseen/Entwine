# Current state

The first public release was v0.1.0. The current source implements a zero-config compiler for `docs/**/*.md`, the
[Entwine Knowledge Convention](convention.md) v0.1, provider-native publishing
setup, and a public showcase built from its own output.

## Implemented commands

| Command | Result |
| --- | --- |
| `entwine init [project]` | Scaffolds missing recommended knowledge files; never overwrites |
| `entwine check [project]` | Diagnostics, knowledge coverage, non-zero exit on errors |
| `entwine check --strict-knowledge` | Also fails when a recommended area is missing |
| `entwine build [project]` | Static docs, copied assets, graph, Knowledge overview in `dist/` |
| `entwine graph [project]` | Identical to `build`; reports the graph page location |
| `entwine context [project]` | Complete deterministic Markdown context |
| `entwine context [project] --json` | Versioned structured JSON context |
| `entwine dev [project] --port 4173` | Watched build and loopback HTTP server |
| `entwine setup [project]` | Generates GitHub, GitLab, or Bitbucket validation and publishing config |

## Supported content

CommonMark headings, paragraphs, emphasis, lists, links, code, blockquotes, GFM
tables, strikethrough, and task lists. Optional YAML `title`, `type`, and `status`
metadata are strings; unknown keys are ignored. Titles fall back to first H1,
then humanized filename. Repeated headings get suffixes starting at `-2`.

Document links, extensionless routes, anchors, and local assets are resolved.
Repeated links yield one document relationship. Orphans are warnings.
`docs/index.md` is canonical; root `docs/README.md` falls back to `/` and
`project` only when the index is absent. With both present, README keeps
`/README/` and `other` unless explicitly typed. Nested READMEs keep ordinary
routes. Documentation directories with descendants receive navigation-only
landing pages when no document already occupies that route.

Links to existing repository files outside `docs/` become repository references,
never knowledge relationships. Paths outside the repository and symlink crossings
fail. Local Git metadata can provide credential-free, encoded file links for
GitHub, GitLab, and Bitbucket; unknown providers or files absent at the local
commit render labeled paths. Repository files are never copied to `dist/`.

## Knowledge roles

Every document has a derived role: project, architecture, state, roadmap,
decision, spec, or other. A recognized `type` wins, then the canonical path, then
`other`. The raw `type` is always preserved. A conflicting `type` and canonical
path warns and never fails. `entwine check` reports which recommended areas are
represented; that is presence, not quality. See the [convention](convention.md).

## Reading, graph, and knowledge views

Documentation uses neutral gray surfaces with automatic light/dark support.
Mobile navigation and page headings use native collapsible menus. Pages show a
role badge, and the sidebar links to two Entwine-native views: the generated
Project knowledge overview at `/__entwine/knowledge/` (what exists) and the graph
(how it is connected). Small graphs center the most connected document. Graphs above twelve nodes
use deterministic role bands and directed curved edges. Above fifty nodes or
two hundred edges, an open role-grouped text index becomes primary; the full
visual graph remains available in a native disclosure. Node names include the role, and document lists are grouped by role, so
meaning never depends on color. Document and relationship lists remain complete. Pages show outgoing references,
backlinks, and Markdown source paths. Large sibling navigation lists show twenty
entries plus the current page/ancestors and a link to the complete Knowledge index.

## Public showcase and distribution

One Cloudflare Pages artifact serves the Astro website at `/`, Entwine's own
documentation at `/docs/`, and the kitchen-sink example at `/demo/`. That
deployment is for Entwine itself; it is separate from the provider-native setup
offered to users. The last two are unmodified `entwine build` output hosted below
a subpath. `pnpm build:showcase` produces the same `deployment/` directory locally
and verifies every relative link, asset, and fragment under both mount points.
Tagged `v*` releases publish native binaries for Linux x86_64, macOS arm64, macOS
x86_64, and Windows x86_64 on GitHub Releases, with SHA-256 checksums, and the
same binaries to npm as `@entwine/cli` plus one `@entwine/cli-<platform>` package
per target. Version 0.1.0 is published on npm and GitHub Releases. Generated CI pins
`@entwine/cli` at the CLI's own version; see [deployment](deployment.md).

## Provider-native publishing

`entwine setup` detects GitHub, GitLab, and Bitbucket Cloud from local Git
remotes without network access and writes isolated, least-privilege configuration.
It supports `--dry-run`, is idempotent, and never overwrites existing CI. The
[generated CI contract](specs/generated-ci.md) lists the guarantees. Unknown
providers get portable `dist/` guidance. Bitbucket publishing needs one-time
manual setup that cannot be automated safely.

## Portability checks

Routes that differ only by case are errors, because macOS and Windows file systems
would merge their output. Tests cover Unix and Windows separators, nested index
routes, traversal, the reserved `__entwine` path, deep trees, long filenames,
repeated and missing headings, README-style Markdown, large code blocks, and
graphs of 1, 5, 20, 50, and 100 nodes.

## Limits

No search, theme configuration, graph dragging or physics, incremental
compilation, or browser reload. Refresh after dev rebuilds. The graph uses a
fixed SVG layout with a complete text relationship list; large graphs prioritize an open role-grouped index, with the full SVG available
on demand. Binaries are unsigned, and Linux builds
link the system glibc. Generated GitLab and Bitbucket pipelines are verified for
syntax and structure but have not been run on those providers; GitLab needs 17.10
or later. A page that links to its own heading counts as referencing itself. Raw
HTML is escaped except empty anchors with a single safe quoted `id` or `name`. Symbolic links are rejected. Builds own `dist/` entirely.
Context schema 0.3 is experimental. API compatibility is not guaranteed before 1.0.

Read the [architecture](architecture.md) and [future roadmap](roadmap.md).

## Provider evidence

| Provider | Generated + YAML/contract tests | Real generated CI | Real native publishing |
| --- | --- | --- | --- |
| GitHub | Yes | No | No |
| GitLab | Yes | No | No |
| Bitbucket Cloud | Yes | No | No |

Entwine’s own GitHub CI, release workflow, and Cloudflare showcase deployment
were verified successful on the v0.1.0 main commit. Those are different from a
consumer’s generated GitHub Pages workflow. No dedicated hosted provider test
repository or GitLab/Bitbucket credentials were used. See
[hardening evidence](hardening.md) for exact validation and measurements.
