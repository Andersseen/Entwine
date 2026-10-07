<div align="center">

<img src="apps/www/public/logo.png" alt="Entwine logo" width="120" />

# 🧶 Entwine

### Project knowledge, connected.

[![CI](https://img.shields.io/github/actions/workflow/status/Andersseen/Entwine/ci.yml?branch=main&style=flat-square&label=ci)](https://github.com/Andersseen/Entwine/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/github/license/Andersseen/Entwine?style=flat-square)](LICENSE)
[![npm version](https://img.shields.io/npm/v/@entwine/cli?style=flat-square)](https://www.npmjs.com/package/@entwine/cli)
![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange?style=flat-square)

</div>

Architecture notes, specs, and decisions live beside your code, but the
connections between them disappear. Entwine is a repository-native project
knowledge compiler: it turns the Markdown already in your repository into one
shared source of project knowledge for humans and agents. It is written in Rust
(with TypeScript for its website and tooling) and has no runtime; your project
can use any stack.

```text
            Markdown
               │
         Knowledge model
        /      |      \
      Docs   Graph   Context
    (humans)  (both)  (agents)
```

Markdown remains the source of truth. Links create relationships. The website is
one projection of project knowledge, the graph is another, and structured
context is a third.

> **Experimental / pre-1.0.** APIs and the context schema may change.

## See it live

Entwine's own showcase is published to Cloudflare Pages. (That is this project's choice; see [provider-native publishing](#publishing-with-your-git-provider) for what your repository gets.)

**Website & demo:** [entwine.andersseen.dev](https://entwine.andersseen.dev)  
**Live demo project:** [/demo/](https://entwine.andersseen.dev/demo/) (fictional `examples/kitchen-sink`)  
**Demo Knowledge overview:** [/__entwine/knowledge/](https://entwine.andersseen.dev/demo/__entwine/knowledge/)  
**Demo graph:** [/__entwine/graph/](https://entwine.andersseen.dev/demo/__entwine/graph/) (pan, zoom, drag, filter, focus)  
**Demo agent knowledge:** [/__entwine/agents/](https://entwine.andersseen.dev/demo/__entwine/agents/) (instructions and skills, published on purpose)

**Binaries:** [GitHub Releases](https://github.com/Andersseen/Entwine/releases)  
**Deployment config:** [apps/www/DEPLOYMENT.md](apps/www/DEPLOYMENT.md)

## What it does

- Static HTML/CSS documentation with automatic nested navigation and heading TOCs
- File-based routes, resolved Markdown links, and automatic backlinks
- An interactive project graph (static SVG first; pan, zoom, drag, filter by kind and role, focus a node's neighbors, deep-link with `?focus=`), a generated Knowledge overview, and role badges
- Optional repository knowledge: with an `entwine.toml`, Entwine discovers `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, and `SKILL.md`, models their scope and links to your docs, and keeps publication a separate, explicit choice
- A predictable convention for project knowledge, with `init`, coverage in `check`, and
  provider-native publishing through `setup`
- Validation of broken links/anchors, collisions, metadata, and unsafe paths
- Deterministic Markdown and versioned JSON context (with knowledge roles) including full source content
- A local HTTP server that rebuilds when documentation changes

Entwine is stack-agnostic: JavaScript, Rust, Python, Go, and any other consumer
technology work identically. Only Entwine's implementation uses Rust and TypeScript.

## Quick start

### Install

Choose your method:

**npm** (recommended — the launcher installs the native binary for your platform):

```sh
npm install --global @entwine/cli
```

📦 **[View on npm](https://www.npmjs.com/package/@entwine/cli)** · Latest: [![npm version](https://img.shields.io/npm/v/@entwine/cli?style=flat-square&label=)](https://www.npmjs.com/package/@entwine/cli)

**From source** (Rust stable required):

```sh
cargo install --path crates/entwine-cli
```

**Binaries** — Download for your platform from [GitHub Releases](https://github.com/Andersseen/Entwine/releases) (Linux x86_64, macOS arm64/x86_64, Windows x86_64). Verify against `SHA256SUMS`. Binaries are unsigned; there is no Homebrew package.

### Use it

In any repository:

```sh
entwine init                      # scaffold the recommended project knowledge
entwine dev                       # read it at localhost:4173; refresh after changes
entwine check                     # validate it and see which knowledge areas exist
entwine setup                     # automate validation + publishing with your Git provider
```

`init` scaffolds questions, never answers: no AI, no code inspection, and it
never overwrites. `check` fails on errors and only reports recommended areas.

**Other commands:**

```sh
entwine build                     # dist/ including graph and Knowledge overview
entwine graph                     # same as build; the graph is always published
entwine check --strict-knowledge  # also fail when a recommended area is missing
entwine context                   # Markdown to stdout
entwine context --json            # structured JSON (schema 0.4) with roles and artifact kinds
```

Every command accepts an optional project directory, defaulting to the current
directory. Dev also accepts `--port`:

```sh
entwine build examples/kitchen-sink
entwine dev examples/kitchen-sink --port 4300
cargo run -p entwine-cli -- context examples/kitchen-sink --json
```

No configuration is required. Defaults are `docs/` and `dist/`; an optional
[`entwine.toml`](#repository-knowledge-optional) only switches on agent knowledge. Output owns the
entire `dist/` tree; an existing directory without Entwine's marker is refused.
Rebuilds remove stale pages and preserve the last successful output on validation
failure. Deploy the resulting directory to any static host, including project
subpaths. Serve it over HTTP for normal directory-index behavior.
Root `docs/README.md` is the fallback when `docs/index.md` is absent. Without
either, Entwine generates a navigation landing page.

## Entwine Knowledge Convention

A small, open, optional convention for durable project knowledge. It is a
proposal that has to earn adoption, not a standard.

```text
docs/
├── index.md          what this is, where things are
├── architecture.md   how it is structured today
├── state.md          what is true right now
├── roadmap.md        what is intended next
├── decisions/        what was decided, and why
└── specs/            what capabilities do
```

**Recommended, not required.** Arbitrary Markdown compiles normally. Entwine
derives a role for each document from a recognized frontmatter `type` first, then
the canonical path, so `docs/design/system.md` with `type: architecture` is
understood as architecture without renaming. `entwine check` shows which areas
exist (presence, never a quality score), and `--strict-knowledge` is the opt-in
for teams that want enforcement. Entwine complements `README.md`, `AGENTS.md`,
`SKILL.md`, MCP, and OpenSpec; see [docs/convention.md](docs/convention.md).

## Repository knowledge (optional)

Durable documentation lives in `docs/`. Repositories also hold agent-facing
files. Entwine can discover and model them next to your documentation. It reads
and structures them; it does not run, evaluate, or resolve them for any tool.
This is off until you add an `entwine.toml` at the project root:

```toml
[discovery]
agent_instructions = true   # AGENTS.md, CLAUDE.md, GEMINI.md at any depth
skills = true               # SKILL.md manifests (folder resources are listed, never run)

[site]
include_agent_knowledge = false   # default; true publishes them in the generated site
```

| What | Default | With `entwine.toml` |
| --- | --- | --- |
| `docs/` compiled to `dist/` | on | on |
| Agent instructions and skills **discovered** | off | per `[discovery]` |
| In `entwine context` and `check` | no | yes, when discovered |
| In the **public site** and graph | no | only if `include_agent_knowledge = true` |

Discovery and publication are separate on purpose: instructions can be internal,
so a file can be in your context without ever reaching a public site. Unknown
keys in `entwine.toml` are errors, so a typo never silently publishes anything.
Instruction files carry a structural **scope** (the directory they sit in), skills
carry their folder and resources, and links between these files and your docs
become graph relationships. Discovery skips `docs/`, `node_modules`, `dist`,
`target`, `.git`, and symbolic links. `entwine init` stays focused on the
documentation convention and does not generate these files. Details:
[agent knowledge](docs/specs/agent-knowledge.md).

## Publishing with your Git provider

`entwine setup` writes provider-native validation and publishing once; after that
your provider's pipeline owns it. Entwine is not a hosted platform and stores no
credentials.

| Repository | Publishes to |
| --- | --- |
| GitHub | GitHub Pages |
| GitLab | GitLab Pages |
| Bitbucket Cloud | Bitbucket static website hosting (needs one manual setup step) |
| Any other Git | `dist/` on any static host |

Pull requests run `entwine check`, including repository-file reference changes; the default branch checks,
builds, and publishes the same artifact. `entwine setup --dry-run` changes
nothing, repeated runs are idempotent, and existing CI is never overwritten.
Details and constraints: [docs/deployment.md](docs/deployment.md). Generated
pipelines install `@entwine/cli` pinned to the CLI's version, available from npm since v0.1.0.

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
are validated and copied with their relative paths. Arbitrary raw HTML is escaped;
empty safe anchor declarations become generated spans. Links outside `docs/` to
existing repository files are separate repository references. Unsafe URL schemes,
symlinks, and paths outside the repository remain errors.

## Architecture

- **entwine-core:** pure serializable domain model, route and title rules, and the
  knowledge-role classification. No I/O.
- **entwine-engine:** scan once, parse once, resolve routes/links, validate, build
  `KnowledgeBase`, then project `SiteModel`, `GraphModel`, and `ContextModel`.
- **entwine-cli:** Clap commands, diagnostics, staged filesystem output, HTTP,
  file watching, `init` scaffolding, Git provider detection, CI templates, and exit codes.
- **apps/www:** the separate static Astro product website, strict TypeScript,
  no frontend framework. It is not the renderer for consumer documentation.

The renderer is not the knowledge model. See [architecture](docs/architecture.md),
[current state](docs/state.md), and [roadmap](docs/roadmap.md).

## What it is not

Entwine is not a CMS, Obsidian clone, task manager, hosted docs platform, AI agent,
agent orchestrator, or specification lifecycle tool. It does not replace
`README.md`, `AGENTS.md`, `SKILL.md`, MCP, or OpenSpec. There is no database, AI,
plugin system, source-code analysis, or runtime required for production docs.

## Development

Use Rust stable, Node 22.18+, and pnpm 10.
Cargo remains authoritative for Rust; Turbo orchestrates website tasks.

```sh
pnpm install --frozen-lockfile
cargo build --workspace
pnpm check
```

`pnpm dev:www` serves `/docs/` and `/demo/` from `deployment/`, so run
`pnpm build:showcase` once first; otherwise those two paths show a hint.

Available shortcuts: `pnpm dev`, `dev:www`, `build`, `build:www`, `test`, `lint`,
`typecheck`, `format`, `format:check`, `build:showcase`, and `check`. Direct gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm lint
pnpm typecheck
pnpm build:showcase   # website + /docs + /demo into deployment/
cargo run -p entwine-cli -- check .
cargo run -p entwine-cli -- build .
cargo run -p entwine-cli -- check examples/kitchen-sink
cargo run -p entwine-cli -- build examples/kitchen-sink
pnpm smoke:setup      # init + setup for every provider in fresh repositories
```

The kitchen-sink is a healthy twelve-document consumer that follows the convention, plus
instruction files and skills discovered through its `entwine.toml`. Dedicated broken fixtures
live in `crates/entwine-engine/tests/fixtures/`. Tests invoke the real compiler and
binary. CI validates both fixtures and Entwine's own documentation.

## Limits

Full rebuilds, manual browser refresh, no search or theme configuration. The graph
page is static SVG with a deterministic, clustered initial layout and a complete
text index; a small script adds interaction (about 20 KB, no dependencies, and
the page works without it). Dragged nodes stay pinned until the layout is reset,
and the live layout is not saved. Above 150 nodes the text index leads and the
visual graph starts collapsed. Routes that differ only by case are errors; `.md` is the
supported Markdown extension. Symbolic links are not supported. Context schema
`0.4` and Rust APIs are experimental.

MIT licensed. Contributions: [CONTRIBUTING.md](CONTRIBUTING.md).
# Website deployment

The official Astro website has a gated GitHub Actions deployment to Cloudflare
Pages. See [setup instructions](apps/www/DEPLOYMENT.md) for the required repository
secrets and Pages project variable.

## Compatibility and verification

`docs/README.md` can be the root entrypoint when `index.md` is absent. Directories
with documents receive generated navigation pages; authored pages win. Safe links
to repository files outside `docs/` remain separate repository references, with
optional provider source links. Empty explicit HTML anchors with one quoted `id`
or `name` are supported; arbitrary HTML remains escaped.

See [hardening evidence](docs/hardening.md) for real-repository findings, performance,
and the exact hosted-provider verification level. Context schema 0.3 adds explicit
anchors and repository references; package versions still follow release-please.
