# Flowview page renderer

## Context

Entwine's built-in Rust renderer assembles ordinary documentation page shells.
Flowview provides a native static API that compiles an embedded template once
and renders it against JSON without adding a JavaScript runtime. This makes a
renderer-only comparison possible without changing Markdown parsing, the
knowledge model, routing, or Graph generation.

The current implementation uses the temporary immutable Git revision
`35f8c3f211b1f70099462860d80dec944a27d501`. A public `flowview-compiler`
crates.io release was not available in the verified registry state. The Git
revision must not become a permanent default-renderer dependency.

## Evidence

- `entwine-core` has no Flowview dependency. Its renderer setting is a small
  configuration enum and defaults to `builtin`.
- The embedded template compiles and renders a nested hostile-string fixture.
  Tests cover default selection, deep routes, escaped metadata, trusted
  Markdown body HTML, and deterministic output.
- Entwine prepares relative URLs in Rust. Recursive navigation remains
  host-rendered with the existing escaping helper. References, backlinks, TOC,
  metadata, and the page shell use structured template input.
- Graph, Knowledge, and Agents remain on their Entwine Rust renderers.
- Same-`SiteModel` page route files and `href` targets match across the two page
  renderers, including long navigation, 15 backlinks and references, and a
  source link. The kitchen-sink fixture passed the same 55 browser tests under
  both renderers.
- Axe reported no violations in either browser run. Mobile and keyboard
  navigation, no-JavaScript output, graph focus links, source links, and console
  error checks passed. Graph output is unchanged because it remains outside the
  page-shell selection.
- Hostile title/navigation labels, metadata, source paths, backlinks, and
  references remain escaped. Entwine-rendered Markdown body HTML is the trusted
  raw body insertion point.
- The `/docs/` and `/demo/` showcase passed the static subpath verifier with
  2,448 relative references resolved. No Flowview runtime or JavaScript is
  emitted.
- On macOS arm64, the release renderer benchmark (median of five renders using
  an already-projected model) was:

  | Pages | Built-in | Flowview | Delta | Built-in bytes | Flowview bytes |
  | ---: | ---: | ---: | ---: | ---: | ---: |
  | 10 | 0.16 ms | 0.67 ms | +0.51 ms | 54,494 | 63,714 |
  | 100 | 1.45 ms | 2.99 ms | +1.54 ms | 404,324 | 496,524 |
  | 500 | 8.76 ms | 9.98 ms | +1.22 ms | 1,961,124 | 2,422,124 |
  | 1,000 | 13.47 ms | 19.37 ms | +5.90 ms | 3,907,124 | 4,829,124 |

- Clean release binaries on this macOS arm64 host measured 3,604,456 bytes for
  the built-in baseline and 4,843,432 bytes with Flowview: +1,238,976 bytes
  (+34.4%). Clean release compilation took 8.5 seconds for the baseline and
  24.6 seconds with Flowview and its 98 additional locked packages. These are
  single-host measurements, not cross-platform CI timings.
- Flowview output is about 23.6% larger at 1,000 pages. The compiler preserves
  template whitespace by design; much of this is the readable indentation and
  line breaks of the embedded template. No whitespace-control syntax is
  available in the pinned compiler, so the output is left readable rather
  than compacting the template or adding a post-render minifier.
- The built-in page renderer is part of a 316-line module. The Flowview adapter
  is 190 lines plus a readable 115-line template. The template clarifies page
  structure, while its mapping adapter and dependency add complexity and cost.
- The locked dependency tree includes Oxc 0.137.0 crates declaring Rust 1.94.0
  as their minimum supported version. Entwine declares Rust 1.85, so the current
  dependency does not meet Entwine's MSRV contract. Rust 1.85 was not installed
  on the host for a direct compiler check.
- CI runs the Flowview-inclusive Rust suite on Linux, macOS, and Windows. The
  browser suite and `/docs/` / `/demo/` subpath verifier run on Linux.
- Navigation stays host-rendered. The template has no recursive partial
  mechanism; structured navigation would require flattening or expanding the
  Flowview language, making the template less direct.

## Decision

**ADOPT HYBRID, default deferred.** Flowview remains an opt-in candidate for
ordinary document page shells. Graph, Knowledge, and Agents remain Entwine Rust
renderers. The built-in renderer remains the default, rollback path, and parity
reference.

The default is deferred for two concrete blockers:

1. No compatible public `flowview-compiler` registry release is available. The
   immutable Git revision remains temporary.
2. The locked Oxc dependency tree declares Rust 1.94.0, exceeding Entwine's Rust
   1.85 MSRV. Do not raise Entwine's MSRV implicitly.

Reconsider the default only after a compatible public registry release exists
and Linux, macOS, Windows, MSRV, semantic parity, browser, Axe, subpath,
escaping, and performance gates pass. No release version is manually forced by
this decision.

## Consequences and rollback

Omitting the renderer setting or choosing `renderer = "builtin"` selects the
built-in path. Removing the Flowview candidate requires deleting the engine
dependency, embedded template, view-model adapter, and selection option; the
canonical knowledge and graph models remain unaffected.
