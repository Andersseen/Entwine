# Experimental Flowview renderer

## Context

Entwine's built-in Rust renderer currently assembles the documentation page
shell. Flowview now provides a native static API that compiles an embedded
template once and renders it against JSON without adding a JavaScript runtime.
That makes a renderer-only comparison possible without changing Markdown
parsing, the knowledge model, routing, or Graph generation.

The crate was not returned by `cargo search --registry crates-io
flowview-compiler` during this experiment. The implementation temporarily pins
Flowview to immutable revision
`35f8c3f211b1f70099462860d80dec944a27d501`; this is not acceptable as a
permanent default-renderer dependency.

## Experiment criteria

- Keep the built-in renderer as the default and parity oracle.
- Restrict Flowview to ordinary documentation page shells in `entwine-engine`.
- Compare the same `SiteModel`, including routes, navigation, metadata, body
  HTML, references, backlinks, and source links.
- Preserve escaping, landmarks, mobile navigation, subpath-relative links,
  deterministic output, and staged-output safety.
- Measure render time, output bytes, binary size, and the template's
  maintainability before choosing a default.

## Evidence so far

- `entwine-core` has no Flowview dependency; its renderer setting is a small
  configuration enum and defaults to `builtin`.
- The embedded template compiled and rendered against a nested hostile-string
  fixture. Focused tests pass for default selection, deep routes, escaped
  metadata, trusted Markdown body HTML, and deterministic output.
- The page renderer prepares relative URLs in Rust. Recursive navigation is
  host-rendered with the existing escaped helper; references, backlinks, TOC,
  metadata, and page shell use structured template input.
- Graph, Knowledge, and Agents remain on their existing Rust renderers.
- Same-`SiteModel` page route files and `href` targets match across the built-in
  and Flowview renderers, including a long navigation tree, 15 backlinks and
  references, and a source link. The kitchen-sink fixture also passes the same
  55 browser tests under both renderers.
- Axe reports no violations in either browser run. Mobile navigation, keyboard
  navigation, no-JavaScript output, graph focus links, source links, and console
  error checks pass. Graph output is unchanged because its Rust renderer is
  outside this page-shell selection.
- Hostile title/navigation labels, metadata, source paths, backlinks, and
  references remain escaped. Entwine-rendered Markdown body HTML remains the
  trusted raw body insertion point.
- The experimental `/docs/` and `/demo/` showcase passes the static subpath
  verifier with 2,448 relative references resolved. No Flowview runtime or
  JavaScript is emitted.
- On this macOS arm64 host, the release renderer benchmark (median of five
  renders, already-projected model) was:

  | Pages | Built-in renderer | Flowview renderer | Built-in bytes | Flowview bytes |
  | ---: | ---: | ---: | ---: | ---: |
  | 10 | 0.15 ms | 0.44 ms | 54,494 | 63,714 |
  | 100 | 1.32 ms | 2.16 ms | 404,324 | 496,524 |
  | 500 | 6.29 ms | 9.43 ms | 1,961,124 | 2,422,124 |
  | 1,000 | 12.38 ms | 18.77 ms | 3,907,124 | 4,829,124 |

- A release binary built from `main` measured 3,604,456 bytes. The Flowview
  build measured 4,850,008 bytes: +1,245,552 bytes (+34.5%). Clean local
  release compilation was about 8.7 seconds for `main` and 28.5 seconds with
  Flowview and its 98 additional locked packages. These are single-host build
  measurements, not cross-platform CI timings.
- The old page rendering shell is part of a 316-line renderer module. The new
  Flowview adapter is 190 lines plus a readable 115-line template. The template
  makes the page structure and repeated sections easier to scan, while the
  mapping adapter and temporary dependency add real complexity and cost.

## Decision

ADOPT HYBRID for the ordinary documentation page shell: Flowview is a viable
template boundary, while specialized Graph, Knowledge, and Agents pages stay in
Rust. Keep the built-in renderer as the default for now. The crate is not
available from crates.io, and its pinned Git dependency is permitted only for
this experiment. A public registry release and CI verification on Linux and
Windows are required before considering Flowview as the default.

## Consequences and rollback

The current output path can switch back by omitting the renderer setting or
choosing `renderer = "builtin"`. Removing the experiment requires deleting the
engine dependency, embedded template, view-model adapter, and selection option;
the canonical knowledge and graph models remain unaffected.
