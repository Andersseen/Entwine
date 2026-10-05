# Entwine

Project knowledge, connected.

**Experimental / pre-1.0 — version 0.1.0.** APIs and context schema may change.

Entwine is a repository-native project knowledge/documentation compiler. Put
Markdown under `docs/`; get useful views without building a documentation app.

```text
docs/**/*.md
     ↓
   Entwine
  /   |    \
Docs Graph Context
```

Markdown remains the source of truth. Links create relationships.

## What it does

- Static HTML/CSS documentation with automatic nested navigation and heading TOCs
- File-based routes, resolved Markdown links, and automatic backlinks
- A linked SVG project graph and readable relationship list
- Validation of broken links/anchors, collisions, metadata, and unsafe paths
- Deterministic Markdown and versioned JSON context including full source content
- A local HTTP server that rebuilds when documentation changes

Entwine is stack-agnostic: JavaScript, Rust, Python, Go, and any other consumer
technology work identically. Only Entwine's implementation uses Rust and TypeScript.

## Quick start

Requires Rust stable. No published binary or package-manager release is promised
yet. From this source checkout:

```sh
cargo install --path crates/entwine-cli
```

From any project containing `docs/index.md`:

```sh
entwine dev                       # localhost:4173; refresh after changes
entwine build                     # dist/ including graph
entwine graph                     # dist/__entwine/graph/index.html
entwine check                     # errors exit non-zero; warnings don't
entwine context                   # Markdown to stdout
entwine context --json            # structured JSON to stdout
```

Every command accepts an optional project directory, defaulting to the current
directory. Dev also accepts `--port`:

```sh
entwine build examples/kitchen-sink
entwine dev examples/kitchen-sink --port 4300
cargo run -p entwine-cli -- context examples/kitchen-sink --json
```

No configuration is required. Defaults are `docs/` and `dist/`. Output owns the
entire `dist/` tree; an existing directory without Entwine's marker is refused.
Rebuilds remove stale pages and preserve the last successful output on validation
failure. Deploy the resulting directory to any static host, including project
subpaths. Serve it over HTTP for normal directory-index behavior.
If there is no `docs/index.md`, Entwine generates a directory landing page.

## Markdown and routing

| File | Route | Output |
| --- | --- | --- |
| `docs/index.md` | `/` | `dist/index.html` |
| `docs/architecture.md` | `/architecture/` | `dist/architecture/index.html` |
| `docs/specs/authentication.md` | `/specs/authentication/` | `dist/specs/authentication/index.html` |
| `docs/specs/index.md` | `/specs/` | `dist/specs/index.html` |

`specs.md` and `specs/index.md` collide. `__entwine/` is reserved. Navigation
follows the directory tree and sorts lexically, with the home page first.

CommonMark, tables, strikethrough, and task lists are supported. Optional metadata:

```yaml
---
title: Authentication
type: spec
status: planned
---
```

Known values must be non-empty single-line strings. Unknown keys are ignored.
Title falls back to the first H1, then a humanized filename. Heading anchors use
lowercase letters/digits and hyphens; repeated headings get `-2`, `-3`, etc.

`[Architecture](../architecture.md#boundaries)` becomes a portable generated URL.
Resolved document links create unique `references` relations; backlinks derive
automatically. External URLs remain untouched and are never fetched. Local assets
are validated and copied with their relative paths. Raw HTML is displayed as text.
Unsafe URL schemes, symlinks, and traversal outside `docs/` are errors.

## Architecture

- **entwine-core:** pure serializable domain model, route and title rules.
- **entwine-engine:** scan once, parse once, resolve routes/links, validate, build
  `KnowledgeBase`, then project `SiteModel`, `GraphModel`, and `ContextModel`.
- **entwine-cli:** Clap commands, diagnostics, staged filesystem output, HTTP,
  file watching, and exit codes.
- **apps/www:** the separate static Astro product website, strict TypeScript,
  no frontend framework. It is not the renderer for consumer documentation.

The renderer is not the knowledge model. See [architecture](docs/architecture.md),
[current state](docs/state.md), and [roadmap](docs/roadmap.md).

## What it is not

Entwine is not a CMS, Obsidian clone, task manager, hosted docs platform, AI agent,
agent orchestrator, or specification lifecycle tool. There is no database, AI,
plugin system, source-code analysis, or runtime required for production docs.

## Development

Use Rust stable, Node 22.12+ (or a newer Astro-supported Node), and pnpm 10.
Cargo remains authoritative for Rust; Turbo orchestrates website tasks.

```sh
pnpm install --frozen-lockfile
cargo build --workspace
pnpm check
```

Available shortcuts: `pnpm dev`, `dev:www`, `build`, `build:www`, `test`, `lint`,
`typecheck`, `format`, `format:check`, and `check`. Direct gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm lint
pnpm typecheck
pnpm build:www
cargo run -p entwine-cli -- check .
cargo run -p entwine-cli -- build .
cargo run -p entwine-cli -- check examples/kitchen-sink
cargo run -p entwine-cli -- build examples/kitchen-sink
```

The kitchen-sink is a healthy seven-document consumer. Dedicated broken fixtures
live in `crates/entwine-engine/tests/fixtures/`. Tests invoke the real compiler and
binary. CI validates both fixtures and Entwine's own documentation.

## 0.1 limits

Full rebuilds, manual browser refresh, no search or theme configuration. SVG graph
layout is fixed, with scrolling and a complete text list for larger/dense projects;
there is no force simulation or zoom UI. Paths are case-sensitive; `.md` is the
supported Markdown extension. Symbolic links are not supported. Context schema
`0.1` and Rust APIs are experimental. The site GitHub links use repository discovery
until this checkout has a public upstream URL.

MIT licensed. Contributions: [CONTRIBUTING.md](CONTRIBUTING.md).
