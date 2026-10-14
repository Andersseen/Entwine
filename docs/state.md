# Current state

The current source implements a zero-config compiler for `docs/**/*.md`, the
[Entwine Knowledge Convention](convention.md) v0.1, provider-native publishing
setup, optional [agent knowledge](specs/agent-knowledge.md) discovery, an
interactive graph, and a public showcase built from its own output.

## Agent integration

`skills/` ships three Agent Skills (`entwine-get-started`, `entwine-knowledge`,
`entwine-deployment`), validated with the `skills-ref` reference validator and
installable with the `skills` CLI. `packages/mcp` (`@entwine/mcp`) is a stdio MCP server
built on `@modelcontextprotocol/server` v2 that wraps the CLI: seven read-only tools,
a summary resource plus per-document resources, and three prompts. It supports context
schema `0.4` only and is versioned in lockstep with `@entwine/cli`. `@entwine/mcp`
and `@entwine/cli` are published on npm at `0.7.0` (verified 12 October 2026, see
[release status](#release-status)); the skills are installed from GitHub with the
`skills` CLI, and no `@entwine/skills` package exists. See
[agent integration](agent-integration.md).

## Release status

Verified on 10 October 2026 from the public GitHub API, Actions runs, and npm:

| Version | GitHub Releases                      | npm `@entwine/cli`     |
| ------- | ------------------------------------ | ---------------------- |
| 0.5.0   | Public                               | Published              |
| 0.5.1   | Public prerelease; not marked latest | Published and `latest` |
| 0.6.0   | Skipped; never released              | Not published          |
| 0.7.0   | Released                             | Published and `latest` |

On 12 October 2026 the registry listed `@entwine/cli`, `@entwine/mcp` and the four
platform packages (`-darwin-arm64`, `-darwin-x64`, `-linux-x64`, `-win32-x64`) all at
`0.7.0`; `@entwine/mcp@0.7.0` depends on `@entwine/cli@0.7.0` exactly. `node
tooling/smoke-public.ts --version 0.7.0` (fresh install, CLI run, MCP handshake)
passed against the real registry. Release PR #44 (`0.8.0`) is untouched.

The `v0.5.1` release targets commit `898ebd79`. Its four native archives and
`SHA256SUMS` were already attached; I downloaded them and verified every archive
against the checksum file. All five npm packages were already public, and the
release recovery workflow reused them without rebuilding or republishing.
Workflow run [38056438278](https://github.com/Andersseen/Entwine/actions/runs/38056438278)
passed its public registry smoke and published the draft on 10 October 2026.
The tag now points to the expected commit. The release is a prerelease and is
not GitHub's `latest`; npm `@entwine/cli` and all four platform packages list
`0.5.1` as `latest`. A fresh isolated consumer install and smoke also passed.

The release guard remains in place: while a draft is pending, a `main` push
skips release-please and downstream release jobs. After `v0.5.1` was
published, the release state is consistent again.

Superseded: `0.7.0` has since been released and published (see the table above).

**Decision (11 October 2026):** `0.6.0` is skipped. Its release PR (#34) merged while
`0.5.1` was still a draft, and GitHub refused to create the release afterwards, so every
`main` push failed in `release-please`. PR #34 is relabelled `autorelease: tagged`, and
`last-release-sha` in `release-please-config.json` anchors the next release PR (`0.7.0`,
which carries the MCP server and skills) at the #34 merge commit, so it lists only commits
after it. Remove `last-release-sha` once `v0.7.0` is published. The `0.7.0` release is the
first real test of release creation; if it fails with the same error, check the
admin-only Actions settings before adding any credential workaround.

The subsequent release-please run [38068919404](https://github.com/Andersseen/Entwine/actions/runs/38068919404)
failed while creating the GitHub release with `Resource not accessible by
integration` from `POST /repos/{owner}/{repo}/releases`. The workflow declares
`contents: write`, and the failed job log confirms that the run's `GITHUB_TOKEN`
reported `Actions: write`, `Contents: write`, and `PullRequests: write`. The
`push` run successfully loaded release-please 17.3.0, found merged PR #34, and
started creating one release before GitHub denied the POST. This rules out a
missing declared `contents: write` grant, but does not identify which GitHub
policy denied the release endpoint. The only repository ruleset visible to the
connected API is a disabled branch rule for `main`; no `v0.6.0` tag, draft, or
public release exists. There were no open PRs at diagnosis time; draft code-fix
PR [#38](https://github.com/Andersseen/Entwine/pull/38) is now open. Admin-only
Actions settings and legacy tag protections were not available through the
read-only GitHub connection. Do not add a credential workaround until those
settings are checked.

Release recovery now downloads existing draft assets, preserves them, validates
the complete `SHA256SUMS` set, and uploads only missing expected filenames. The
upload no longer uses `--clobber`, so a retry cannot replace an attached asset.
Resume also now fails closed if GitHub cannot verify a tag, resolves annotated
tags, and rejects a tag that points to a different commit. These safeguards are
covered by mocked release-tooling tests and passed the PR CI run
[38073298676](https://github.com/Andersseen/Entwine/actions/runs/38073298676).
They have not yet been exercised against a live release.

### Release re-check, 11 October 2026

Release run [38074664259](https://github.com/Andersseen/Entwine/actions/runs/38074664259)
on `314bc95` (which contains the #38 hardening) passed its guard and then failed
in `release-please` with the same `Resource not accessible by integration` from
`POST /repos/{owner}/{repo}/releases`. Read-only API checks show: no `v0.6.0`
release or tag, npm `latest` still `0.5.1`, default workflow permissions `read`,
one repository ruleset (`Protect Main`) disabled, immutable releases disabled.
The cause is not visible to a read-only connection; an owner must inspect
Settings → Actions and any organization policy. Nothing was published and no
credential was introduced.

### Release re-check, 11 October 2026 (later)

`v0.6.0` is still not public: no GitHub release or tag, npm `latest` is `0.5.1`.
The `Release` run for `3fc593e` failed in `release-please` with the same
`Resource not accessible by integration`. The `CI` run for `3fc593e` also failed
on `portability (macos-14)`: the Bitbucket publish test cloned a local bare
repository with default hardlink/copy semantics, which fails on that runner. The
test now clones with `--no-local`.

## Deployment validation

Generated pipelines and portable output are verified locally; no live provider
deployment has been run. See [deployment validation](deployment-validation.md).

## Implemented commands

| Command                             | Result                                                                                               |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `entwine init [project]`            | Scaffolds missing recommended knowledge files; never overwrites                                      |
| `entwine check [project]`           | Diagnostics, knowledge coverage, discovered agent knowledge when configured, non-zero exit on errors |
| `entwine check --strict-knowledge`  | Also fails when a recommended area is missing                                                        |
| `entwine build [project]`           | Static docs, copied assets, graph, Knowledge overview in `dist/`                                     |
| `entwine graph [project]`           | Identical to `build`; reports the graph page location                                                |
| `entwine context [project]`         | Complete deterministic Markdown context                                                              |
| `entwine context [project] --json`  | Versioned structured JSON context (schema 0.4), including discovered agent artifacts                 |
| `entwine dev [project] --port 4173` | Watched build and loopback HTTP server                                                               |
| `entwine setup [project]`           | Generates GitHub, GitLab, or Bitbucket validation and publishing config                              |

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

## Optional configuration and repository knowledge

`entwine.toml` is optional. Without it, only `docs/` is compiled and nothing
agent-facing is discovered or published. With it, `[discovery]` turns on
`agent_instructions` (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`) and `skills`
(`SKILL.md`), and `[site] include_agent_knowledge` separately decides whether
they reach the generated site. Discovered files are always present in
`entwine context` and reported by `entwine check`; they are public only when
publication is explicitly enabled. `[site] public_agent_paths` (source-only, after
0.7.0) publishes an exact list of discovered artifacts instead of all of them; the
filter is applied at the projection boundary, so context, `check` and the MCP server
still see everything. Unknown keys are errors. See
[agent knowledge](specs/agent-knowledge.md).

The optional `[site] renderer = "flowview"` setting selects the experimental
Flowview page-shell renderer. The default build excludes its compiler and
supports Rust 1.85; Flowview builds use `--features flowview-renderer` and
currently require Rust 1.94 or newer. Graph, Knowledge, and Agents pages keep
their existing Rust renderers. The compiler remains pinned to an immutable Git
revision because crates.io returned no `flowview-compiler` crate on 10 October
2026. See the [experiment record](decisions/flowview-renderer-experiment.md)
for parity criteria and current evidence.

Each document has a role (what it means) and an artifact kind (`documentation`,
`agent_instructions`, `skill`). Instruction files carry a structural scope (the
directory they sit in); skills carry folder, `name`, `description`, and listed
resources. Links between these files and documentation become relationships.
Published pages live under `/__entwine/agents/` (overview, instructions, skills,
scopes, one page each). `entwine init` does not scaffold these files.

## Reading, graph, and knowledge views

Documentation uses neutral gray surfaces with automatic light/dark support.
Mobile navigation and page headings use native collapsible menus. Pages show a
role badge, and the sidebar links to two Entwine-native views: the generated
Project knowledge overview at `/__entwine/knowledge/` (what exists) and the graph
(how it is connected).

The graph page is static HTML with an inline SVG laid out at build time by a
deterministic force-directed layout, clustered by artifact kind and role. Shape
shows the kind (circle documentation, square instructions, diamond skill, dashed
square repository file), letters show the role, and colors are secondary, so
meaning never depends on color. A small dependency-free script
(`__entwine/graph.js`, about 20 KB) adds pan, wheel/pinch zoom, node dragging
(dragged nodes stay pinned until Reset layout), Fit, Reset layout, filters by
kind (documentation, instructions, skills, repository references, off by default)
and by role, a detail inspector (kind, role, source, scope, in/out/repository
counts, clickable neighbors, open page), neighbor highlighting, a Focus mode
that hides everything but a node and its neighbors, and `?focus=<id>` deep links
(pages link to their own node). Labels hide when zoomed out. Without the script
the page still shows the SVG, the legend, and the complete grouped document
index, relationship list, and referenced repository files. Above 150 nodes or 600
edges the text index leads and the visual graph starts collapsed. The role
badges, document lists, and relationship list remain complete. Pages show outgoing references,
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
per target. See [release status](#release-status). Generated CI pins
`@entwine/cli` at the CLI's own version; see [deployment](deployment.md).

## Official website and the Andersseen UI packages

`apps/www` (the Astro site at the root of the Cloudflare Pages deployment) uses the
author's framework-agnostic UI packages, pinned exactly: `@andersseen/layout@0.0.1`
(`and-layout` / `and-text` attributes, CSS only), `@andersseen/icon@0.2.0` (selective
CSS-only icons, nine rules, no runtime), `@andersseen/motion@0.2.0` (`fade-in-up` on
three cards and one transcript; honours `prefers-reduced-motion`; content is never
hidden before JavaScript runs) and `@andersseen/web-components@0.5.1` (`and-tabs` for
Codex / Claude Code, `and-badge` for the version). Only five custom elements are bundled
and they are loaded as lazy chunks when the agent section nears the viewport; without
JavaScript every command is visible and the copy buttons stay hidden. The generated
Entwine documentation depends on none of this: it is built by the Rust renderer and has
no Astro, Stencil or CDN dependency.

Measured on `pnpm --filter @entwine/www build` (gzip): before, HTML 4.3 KB, CSS 1.9 KB,
no JavaScript; after, HTML 6.1 KB, CSS 26.3 KB (layout 9, motion 4, components 8, site 5),
2.8 KB eager JavaScript and about 31 KB of lazy component chunks.

Upstream findings, not worked around by editing those repositories:

- `@andersseen/astro@0.0.2` was published without its `dist/` directory (the tarball
  holds only `package.json`, `README.md`, `LICENSE`), so it cannot be imported.
  `0.0.1` has `dist/`, but it injects an eager page script for all components and
  every icon, which conflicts with lazy loading. Entwine therefore registers its five
  elements itself, with the per-component entry points the integration would use.
- `@andersseen/web-components@0.5.1` ships its stylesheet with the full Tailwind
  preflight (global `body`, `p`, `a`, heading and list resets) and `:host-context()`
  rules that make CSS minifiers warn. `astro.config.ts` strips both by marker and
  fails the build if the marker moves.
- When defined after the page is parsed, `and-tabs` leaves every panel hidden until its
  value is re-assigned (`selectInitialTab` in `src/scripts/site.ts`), and `and-accordion`
  intermittently leaves triggers without `aria-expanded` (about 1 in 25 loads). The
  troubleshooting disclosure therefore uses native `<details>`.

The "words glued to code" defect (`andentwine`, `theentwine CLI`, `approval.Integration
guide`) was Astro's default `compressHTML`, which drops a newline-leading space before
an inline tag. `compressHTML: false` fixes it at the source, and `e2e/www.spec.ts`
checks every inline `code` and link for glued neighbours.

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

No search, theme configuration, incremental compilation, or browser reload.
Refresh after dev rebuilds. The live graph layout is not saved, the simulation
is a light approximation of the build-time layout, and agent instruction scope
is structural only: Entwine does not simulate which file any tool loads.
Agent discovery ignores `.gitignore`. Binaries are unsigned, and Linux builds
link the system glibc. Generated GitLab and Bitbucket pipelines are verified for
syntax and structure but have not been run on those providers; GitLab needs 17.10
or later. A page that links to its own heading counts as referencing itself. Raw
HTML is escaped except empty anchors with a single safe quoted `id` or `name`. Symbolic links are rejected. Builds own `dist/` entirely.
Context schema 0.4 is experimental. API compatibility is not guaranteed before 1.0.

Read the [architecture](architecture.md) and [future roadmap](roadmap.md).

## Provider evidence

| Provider        | Generated + YAML/contract tests | Real generated CI | Real native publishing |
| --------------- | ------------------------------- | ----------------- | ---------------------- |
| GitHub          | Yes                             | No                | No                     |
| GitLab          | Yes                             | No                | No                     |
| Bitbucket Cloud | Yes                             | No                | No                     |

Entwine’s own GitHub CI, release workflow, and Cloudflare showcase deployment
were last verified on the v0.1.0 main commit; they have not been re-verified for
the changes on this branch. Those are different from a
consumer’s generated GitHub Pages workflow. No dedicated hosted provider test
repository or GitLab/Bitbucket credentials were used. See
[hardening evidence](hardening.md) for exact validation and measurements.
