# Flowview as the only page renderer

Status: **proposed** in an unmerged pull request. Nothing here is released until
it ships through the normal release-please process.

This follows [the Flowview renderer experiment](flowview-renderer-experiment.md),
which kept the built-in renderer as the default. Read that record for why.

## Decision

Flowview renders every ordinary documentation page. The built-in page shell, the
`flowview-renderer` Cargo feature, and the renderer-selection plumbing are removed.
There is exactly one page renderer.

## Why

The experiment kept two page shells in sync. The remaining value of that split was
rollback and parity evidence, and the parity evidence is now captured as permanent
tests (below). Keeping a second full renderer only for caution would keep the
duplication the experiment set out to measure.

## What stays in Entwine

Discovery, Markdown parsing and its raw-HTML policy, frontmatter, `KnowledgeBase`,
roles and artifact kinds, agent discovery and publication policy, `SiteModel`,
`GraphModel`, `ContextModel`, routing and URL resolution, anchors, navigation data,
backlinks, repository references, assets, the Graph, Knowledge and Agents pages, the
CLI, dev server, configuration, providers, and deployment.

Flowview owns one thing: turning the prepared page into HTML with
`crates/entwine-engine/src/templates/page.flow`. It does no routing or parsing and
holds no Entwine concepts. Output is plain static HTML with no Flowview runtime.

## The page contract

- `page.flow` is the canonical ordinary-page shell. It is compiled once per site
  render with `compile_static` and reused for every page.
- `flowview_renderer.rs` builds a small typed, internal `PageTemplateContext`
  (site, assets, page, views, home link, navigation). Nothing about it is a public
  schema. Collections keep Entwine's sorted order, so output is deterministic.
- Template and render errors name the affected route.

## Escaping boundary

Every textual value is interpolated with `{{ }}` and escaped by Flowview: titles,
project and navigation labels, metadata, roles, paths, source URLs, heading text,
backlink and reference labels. Exactly two values use raw `{{{ }}}`, both generated
by Entwine:

1. `page.body_html`: Markdown rendered under Entwine's existing raw-HTML policy.
2. `navigation_html`: recursive navigation built by `render::nav`, which escapes
   every label and href. The template language has no recursion, and navigation was
   not flattened to work around that.

Flowview is not a sanitizer. Entwine keeps ownership of input and HTML safety.
`tests/flowview_renderer.rs` injects markup and attribute-breaking strings into every
host-supplied field and asserts none becomes executable HTML.

## Configuration

`[site] renderer` is gone as a choice.

- Absent: the only mode.
- `"flowview"`: still parsed as a deprecated alias. `entwine check` and `build` warn
  "site.renderer is deprecated and ignored" and render normally.
- `"builtin"`: rejected at parse time, with a line number: "no longer supported ...
  Delete the `renderer` key."
- Any other value, or any other unknown key: rejected as before.

The alias selects nothing; it exists so projects that opted in during the experiment
keep building. Delete it in a later release.

## Dependency

`flowview-compiler` is **not published on crates.io** (the registry API returned
"does not exist" for `flowview`, `flowview-compiler`, and `flowview_compiler` on
14 October 2026). The npm release process of Flowview does not imply a Rust crate.
The dependency is therefore an immutable Git revision of Flowview `main`:

```text
77a94d58b44f07bf70c8320ef84341481446cceb
```

It replaces `35f8c3f2`. The new revision includes the UTF-8-safe diagnostic spans,
compiler source maps, adversarial hardening, and static-API test additions made since.
Cargo.lock pins it. Consequence to weigh: a Git dependency prevents publishing
`entwine-engine` or `entwine-cli` to crates.io until Flowview publishes a crate.
Neither is published to crates.io today; the CLI ships as npm packages with native
binaries. This is the same condition the experiment named as a blocker; it is now
accepted rather than resolved.

## Rust version

Raised from 1.85 to **1.94**. Oxc 0.137 (Flowview's parser) declares 1.94.0 and is the
highest `rust-version` in the lockfile. Verified by building, testing and linting the
workspace with `rustc 1.94.0`, and by Cargo rejecting `1.93.1`. The CI `msrv` job now
uses 1.94.0. Users of the precompiled npm binaries are unaffected; only people building
from source need Rust 1.94.

## Evidence

Baseline: the unmodified builtin renderer on `main` (`d719b75`).

- **Paths and links.** Builds of Entwine's own docs (22 files), the kitchen-sink
  (32 files) and a fresh Strata checkout (48 files, read-only) produced identical
  file lists and identical `href` sequences. After normalizing whitespace and `/>`
  on void elements, all HTML files were equal.
- **Permanent tests.** `tests/baseline/` holds the builtin output for the projection
  fixture and the per-page `href`/`aria-label` lists of the kitchen-sink. Tests assert
  Flowview still produces them. The old renderer is not kept to run them.
- **Browser.** The full Playwright suite (76 tests: graph, accessibility with axe,
  privacy and selective agent publication, no-JavaScript, mobile and keyboard
  navigation, reduced motion, the Astro site) passes against the default build. The
  showcase verifier resolved 2,690 relative references under `/docs/` and `/demo/`.
- **Distribution.** `smoke:npm`, `smoke:setup`, `smoke:mcp`, `test:mcp` and
  `test:skills` pass on macOS arm64 with the default build.
- **Differences.** Whitespace changes inside the template; void elements now close
  with `/>`; no change in structure, links, or text.

### Performance

macOS arm64 (Apple M4), Rust 1.99, release, median of seven renders of an
already-projected model. "Total" includes the single template compile.

| Pages | Builtin (before) | Flowview total | Compile only | Per page | HTML bytes (builtin → Flowview) |
| ----: | ---------------: | -------------: | -----------: | -------: | ------------------------------: |
|    10 |          0.16 ms |        0.39 ms |      0.16 ms |  22.8 µs |                 54,494 → 63,714 |
|   100 |          1.45 ms |        2.19 ms |      0.27 ms |  19.2 µs |               404,324 → 496,524 |
|   500 |          6.65 ms |        9.89 ms |      0.15 ms |  19.5 µs |             1,961,124 → 2,422,124 |
| 1,000 |         13.05 ms |       19.49 ms |      0.15 ms |  19.3 µs |             3,907,124 → 4,829,124 |

Builtin figures are from the same machine immediately before the migration. Per-page
cost is about 19 µs and flat; compile cost is paid once. Rendering 1,000 pages costs
about 6 ms more than before, negligible next to Markdown parsing and disk output.

Output is about 24% larger because the template's indentation is preserved. No
minifier was added.

**Native release binary** (macOS arm64, `cargo build --release`): 3,729,256 bytes
before, 4,915,960 after, +1,186,704 bytes (+31.8%). This is the cost of linking Oxc.
Release build time was not measured on a cold cache, so no build-time claim is made.
Linux and Windows sizes come from CI.

### Not demonstrated locally

Linux, Windows and the `macos-14` runner were not available here. Their results come
from GitHub Actions on the pull request.

## Rollback

`main` before this change is `d719b75d1dae877ee964110e3ec584f43206d3d8`, which uses the
built-in renderer with Flowview behind `--features flowview-renderer` at
`35f8c3f211b1f70099462860d80dec944a27d501`. Reverting the merge commit restores both.
No runtime flag keeps the old renderer alive.
