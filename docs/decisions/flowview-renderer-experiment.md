# Flowview page renderer

This record has three parts, oldest first: the first opt-in experiment against a
Git revision (10 October 2026), the brief Flowview-only default (PR #53, reverted
by PR #54), and the second experiment against the published `flowview-compiler` 0.2.0
crate (14 October 2026). The earlier sections are kept unchanged as history; the
[second experiment](#second-experiment-flowview-020-from-cratesio-14-october-2026)
holds the current evidence and decision.

## First experiment: Git revision 35f8c3f (10 October 2026)

### Context

Entwine's built-in Rust renderer assembles ordinary documentation page shells.
Flowview provides a native static API that compiles an embedded template once
and renders it against JSON without adding a JavaScript runtime. This makes a
renderer-only comparison possible without changing Markdown parsing, the
knowledge model, routing, or Graph generation.

The current implementation uses the temporary immutable Git revision
`35f8c3f211b1f70099462860d80dec944a27d501`. The crates.io API returned 404 for
`flowview-compiler` on 10 October 2026, so the Git revision remains necessary.
The Git revision must not become a permanent default-renderer dependency.

### Evidence

- `entwine-core` has no Flowview dependency. Its renderer setting is a small
  configuration enum and defaults to `builtin`.
- Flowview is an optional `entwine-engine` dependency behind
  `flowview-renderer`; `entwine-cli` forwards the same feature. The default
  workspace build and native release binaries omit Flowview. Explicitly
  selecting it in a default build returns an actionable error instead of
  falling back silently.
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

- On 10 October 2026, release binaries rebuilt from the current lockfile on
  this macOS arm64 host measured 3,612,504 bytes for the default build and
  4,843,800 bytes with `flowview-renderer` (+1,231,296 bytes, or 34.1%). The
  local toolchain's `rust-objcopy` could not load `libLLVM.dylib` to strip debug
  info, so applying macOS `strip -S` to copies measured 3,553,680 and 4,783,792
  bytes respectively (+1,230,112 bytes, or 34.6%). The default dependency tree
  and binary exclude Flowview; the experimental binary retains the additional
  compiler cost. For comparison, the earlier combined build measured 3,604,456
  and 4,843,432 bytes respectively. These are single-host measurements, not
  cross-platform CI timings.
- Flowview output is about 23.6% larger at 1,000 pages. The compiler preserves
  template whitespace by design; much of this is the readable indentation and
  line breaks of the embedded template. No whitespace-control syntax is
  available in the pinned compiler, so the output is left readable rather
  than compacting the template or adding a post-render minifier.
- The built-in page renderer is part of a 316-line module. The Flowview adapter
  is 190 lines plus a readable 115-line template. The template clarifies page
  structure, while its mapping adapter and dependency add complexity and cost.
- The locked Flowview dependency tree includes Oxc 0.137.0 crates declaring
  Rust 1.94.0 as their minimum. Entwine still declares Rust 1.85. A direct build
  and test of the default workspace configuration passed with `rustc 1.85.0`
  on 10 October 2026, and its normal dependency tree contains no Flowview/Oxc
  packages. Flowview-enabled builds are intentionally outside that MSRV
  promise.
- CI checks the default build and tests on Rust 1.85, keeps stable Rust quality
  checks, and runs the experimental Flowview parity tests on Linux stable.
  macOS and Windows portability jobs test the default product. Browser E2E and
  `/docs/` / `/demo/` showcase checks run on Linux.
- Navigation stays host-rendered. The template has no recursive partial
  mechanism; structured navigation would require flattening or expanding the
  Flowview language, making the template less direct.

### Decision

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

### Consequences and rollback

Omitting the renderer setting or choosing `renderer = "builtin"` selects the
built-in path. Removing the Flowview candidate requires deleting the engine
dependency, embedded template, view-model adapter, and selection option; the
canonical knowledge and graph models remain unaffected.

## Flowview as the only renderer: #53 and #54

PR #53 made Flowview the only page renderer. It removed the built-in shell, the
`flowview-renderer` feature and the renderer setting, pinned Flowview to Git
revision `77a94d58b44f07bf70c8320ef84341481446cceb`, and raised Entwine's MSRV
to Rust 1.94 for Oxc 0.137. It measured a 3,729,256 → 4,915,960 byte macOS
arm64 release binary (+1,186,704 bytes, +31.8%), about 24% more raw HTML, and
about 19 µs per page. It was merged as `d2bddcc` and reverted by #54
(`5e9855b`), because a Git dependency, a 1.94 MSRV and the binary cost had been
accepted rather than resolved. The second experiment rebuilds the comparison on
current `main` (`feeae5e`) with both renderers kept.

## Second experiment: Flowview 0.2.0 from crates.io (14 October 2026)

### Question

The first experiment already showed that Flowview can render Entwine pages. The
question here is whether, after Flowview 0.2.0's consumer-readiness work,
replacing the built-in page shell is worth what it still costs.

### What changed in Entwine

- `flowview-compiler = "0.2.0"` from crates.io replaces the Git revision. It is
  still optional behind `flowview-renderer`. `Cargo.lock` records
  `registry+https://github.com/rust-lang/crates.io-index` and has no `git+`
  sources. No `[patch]`, path or vendored copy is used. Flowview was not
  modified.
- The adapter builds a small private typed `PageContext` that derives
  `Serialize` and is rendered with `CompiledStaticTemplate::render_serializable`.
  This replaces the hand-written `serde_json::json!` mapping. It only makes the
  adapter easier to read: it still builds a `serde_json::Value` for every page,
  is not zero-copy, and is not faster. The shared `PreparedLink`,
  `PreparedHeading` and `PreparedMetadata` derive `Serialize` only when the
  feature is on. No Flowview type reaches `entwine-core`, `SiteModel` or a
  public API.
- `has_headings`, `has_references`, `has_backlinks`, `has_source` and
  `source.has_url` were removed. The template uses `.length > 0` for
  collections, because an empty array is truthy. It checks
  `context.page.source` for the optional source, and
  `context.page.source.url != null` so an empty URL still behaves like the
  built-in `Some(url)`. `references_collapsible` and `backlinks_collapsible`
  were kept. They now share `COLLAPSED_LINKS_AFTER` with the built-in renderer,
  so the threshold of 12 is not duplicated in the template.
- The template is still compiled once per site render with `compile_static` and
  then rendered for every page. Entwine does not call Flowview's CLI, Node, WASM
  or JavaScript.
- Navigation is still recursive HTML rendered by Entwine. Flowview 0.2.0 still
  has no components, partials, recursion or imports. That is a known design
  topic for a later Flowview version, not a regression, and the navigation was
  not flattened. Only two values are inserted raw: `page.body_html` (Markdown
  rendered by Entwine) and `navigation_html` (escaped by `render::nav`).
- New permanent evidence:
  - `tests/baseline/kitchen-sink-pages.json` was captured from the current
    built-in renderer. It records each kitchen-sink page's hrefs, aria-labels
    and element sequence. The default build checks the built-in renderer
    against it, and the feature build checks Flowview against the same file.
    It survives removal of either renderer.
  - The feature build also checks Flowview against the existing reviewed
    `tests/golden/` page snapshots after normalizing whitespace.
  - `examples/renderer_parity.rs` renders any project with both renderers
    without writing to it, and compares the results.
- The benchmark now also reports the cost of compiling once and the cost per
  page.

### Method

Measured on macOS 26.7.1, Apple M4. "Stable" is `rustc 1.99.0`. Both binaries
in each comparison came from the same source tree, profile and toolchain. The
default release profile was used, with no linker or profile changes.

### Distribution and publishability

- `cargo info flowview-compiler@0.2.0` resolves the crates.io package, which
  declares `rust-version = 1.91`.
- `cargo tree -p entwine-engine --features flowview-renderer` shows
  `flowview-compiler v0.2.0` and `oxc_* v0.116.0` from the registry.
- `cargo package` for `entwine-engine` and `entwine-cli` fails on `main` and on
  this branch, for a reason unrelated to Flowview. The internal `entwine-core`
  and `entwine-engine` path dependencies have no `version`, and Entwine does not
  publish crates today. To isolate the Flowview question, a scratch copy added
  `version = "0.8.0"` to those path dependencies and ran
  `cargo package --workspace --locked`:
  - With the old Git dependency, packaging was rejected with "dependency
    `flowview-compiler` does not specify a version".
  - With 0.2.0, all three crates packaged and verified. The packaged
    `entwine-engine` manifest declares an optional
    `flowview-compiler = "0.2.0"` and includes `templates/page.flow`.

  **The Git-dependency publication blocker is resolved.**

### Rust versions

- Default Entwine: Rust 1.85, unchanged. With a fresh target directory,
  `cargo +1.85.0 build --workspace --locked` built no `flowview` or `oxc`
  artifact, and `cargo +1.85.0 test --workspace --locked` passed.
- With the Flowview feature: Rust 1.91. On `1.91.0`, the Flowview tests, the
  debug and release builds, clippy with `-D warnings`, the showcase and the
  browser suite all passed. On `1.90.0`, Cargo rejects the feature build
  because `flowview-compiler@0.2.0` and `oxc_*@0.116.0` require 1.91.
- The Flowview minimum dropped from 1.94 to 1.91. Adopting Flowview as the
  default or only renderer would still raise Entwine's MSRV from 1.85 to 1.91.

### Binary size

These are macOS arm64 release builds of `entwine-cli` from this branch.

| Toolchain | Built-in | Flowview | Delta |
| --- | ---: | ---: | ---: |
| stable, Flowview 0.2.0 (registry) | 3,575,728 | 4,683,808 | +1,108,080 (+31.0%) |
| 1.91.0, Flowview 0.2.0 (registry) | 3,498,672 | 4,611,376 | +1,112,704 (+31.8%) |
| stable, Git `35f8c3f` (`main`, same day) | 3,575,728 | 4,787,904 | +1,212,176 (+33.9%) |
| stable, local symbols stripped (`strip -x`), 0.2.0 | 2,959,512 | 3,873,736 | +914,224 (+30.9%) |

Moving to Oxc 0.116 saved about 104 KB. The remaining cost is still about
1.1 MB, or about 31%, and every npm user downloads it. This matches Flowview's
own finding that a minimal static consumer costs about 1.2 MB of Oxc.

### Rendering performance

The release `renderer_benchmark` takes the median of seven renders of an
already projected model. Three runs were made on stable; the third is shown,
and the other two were within noise.

| Pages | Built-in | Flowview total | Flowview per page | Delta |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 0.38 ms | 0.80 ms | 38.6 µs | +0.42 ms |
| 100 | 2.13 ms | 3.30 ms | 28.8 µs | +1.17 ms |
| 500 | 8.38 ms | 11.20 ms | 21.6 µs | +2.82 ms |
| 1,000 | 15.74 ms | 22.74 ms | 22.3 µs | +7.00 ms |

Compiling the template once took about 0.42 ms, measured as a render with no
pages. The same benchmark with the Git revision gave 15.98 ms against 22.86 ms
at 1,000 pages, so 0.2.0 renders at the same speed. The difference is
negligible next to Markdown parsing and disk I/O. This was not a blocker before
and is not one now.

### Output size

Each HTML file was compressed on its own and the results were summed (gzip
level 9, Brotli quality 11). "Pages" means documentation pages only. "Site"
means every generated file, including the identical Graph, Knowledge and
Agents pages and the assets.

| Corpus | Scope | Raw | gzip | Brotli |
| --- | --- | ---: | ---: | ---: |
| Entwine docs | pages (17) | 203,676 → 229,343 (+12.6%) | 59,543 → 62,271 (+4.6%) | 47,659 → 50,052 (+5.0%) |
| Entwine docs | site (21) | +8.9% | +3.5% | +3.8% |
| kitchen-sink | pages (16) | 84,240 → 105,272 (+25.0%) | 21,021 → 23,117 (+10.0%) | 15,820 → 17,683 (+11.8%) |
| kitchen-sink | site (31) | +10.0% | +4.2% | +4.8% |
| Strata (read-only, `5537b7e`) | pages (45) | 1,214,101 → 1,286,949 (+6.0%) | 303,271 → 311,103 (+2.6%) | 247,203 → 254,444 (+2.9%) |
| Strata | site (49) | +5.2% | +2.4% | +2.7% |

The extra bytes come from the template's indentation and line breaks, which
Flowview preserves on purpose. Compression removes most of them: on pages with
real content, a reader downloads about 160 to 175 more gzip bytes per page. The
kitchen-sink's short pages show the worst case, at +10% gzip. This is not a
material transfer cost. No minifier was added, and Flowview's whitespace
behavior was not changed.

### Correctness

- **Corpus parity.** `renderer_parity` built Entwine's own docs, the
  kitchen-sink and a read-only Strata checkout with both renderers. In every
  case the generated file lists were identical, and every non-HTML file was
  byte-identical. After normalizing whitespace and `/>`, every HTML file was
  equal:

  | Corpus | Files | HTML | Equal | hrefs | aria-labels |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | Entwine docs | 21 | 19 | 19 | 1,331 | 185 |
  | kitchen-sink | 31 | 28 | 28 | 1,364 | 194 |
  | Strata | 49 | 47 | 47 | 6,290 | 482 |

  A separate tokenizer, Python's `html.parser`, compared tags, attributes and
  text in order and found no differences. Equality covers routes, hrefs,
  aria-labels, structure, navigation, TOCs, references, backlinks, source
  links, assets and relative subpath URLs. Strata's working tree was not
  changed.
- **Hostile values.** A test puts markup, attribute-breaking double and single
  quotes, `</script>`, pre-escaped entities, `&`, `<` and `>` into the title,
  project and nested navigation labels, metadata kind and status, heading text
  and ids, graph focus id, source path, source URL `href`, and backlink and
  reference labels. After normalization, both renderers produce the same output
  for every page, and Flowview's output contains no injected tag or event
  handler. Flowview escapes `'` as `&#39;`, like the built-in renderer.
- **Unicode.** Accented Latin, CJK, right-to-left Hebrew, emoji and ZWJ emoji
  sequences pass through unchanged in text and attributes.
- **Trust boundaries.** A test checks that the Markdown body is inserted
  exactly as Entwine rendered it.
- **Regression check.** A one-word aria-label change in `page.flow` failed
  three of the four baseline tests.
- **Browser.** With `ENTWINE_RENDERER=flowview` on Rust 1.91, the showcase
  verifier resolved 2,727 relative references under `/docs/` and `/demo/`. The
  full browser suite passed all 76 tests, including axe, mobile and keyboard
  navigation, reduced motion, no-JavaScript, source and graph links, privacy,
  selective agent publication and console errors. The default renderer also
  passed all 76.
- **Portability.** CI runs the Flowview tests and debug build on Linux, macOS
  and Windows with Rust 1.91. Release, clippy, showcase and browser checks run
  on Linux only. CI_RESULT_PENDING

### Maintainability

- `page.flow` is 115 readable lines whose shape matches the HTML it produces.
  The built-in shell it would replace is 117 lines: `render_site` plus
  `related_section`. Most of that markup sits in a single `format!` string, and
  every value needs its own `escape(...)` call. Flowview escapes by default, so
  forgetting to escape is no longer a way to introduce a bug.
- The adapter is 153 lines, down from 166, and no longer contains a hand-written
  JSON mapping. About 50 of those lines are type declarations. Replacing the
  built-in shell would therefore add about 150 lines of Rust and template
  overall. It would not remove code.
- The `page_*` helpers, routing, relative URLs, recursive navigation and
  escaping helper stay in Entwine whichever renderer is used.
- Contributors would need to learn one more template language for one page
  shell that rarely changes.

### Scorecard

| Area | Result |
| --- | --- |
| Distribution | Resolved: registry dependency; the workspace packages and verifies. |
| Compatibility | Default build stays on Rust 1.85 and is isolated. Flowview path needs 1.91, down from 1.94. Adopting it raises the MSRV by six minor versions. |
| Binary cost | Unresolved: +1.11 MB (+31%) on every native binary. |
| Output cost | Acceptable: +2.4% to +4.6% gzip and +2.7% to +5.0% Brotli on real sites; +10% to +12% on tiny template-heavy pages. |
| Runtime and build | Acceptable: 0.42 ms compile once, about 22 µs per page, +7 ms at 1,000 pages. |
| Correctness | Proven: semantic parity on three corpora, hostile values, Unicode, browser, axe and subpaths. |
| Maintainability | Modest gain: a readable, escape-by-default template, at the cost of slightly more code, a second language and an external 0.x dependency. |

### Decision

**KEEP EXPERIMENTAL (B).** Flowview 0.2.0 resolves the two blockers named in the
first experiment: there is now a crates.io release, and Flowview no longer
needs Rust 1.94. Parity, safety, portability and performance are proven. The
default is still not changed. What remains is what Flowview costs by design,
not an integration problem:

1. Every native binary grows by about 1.1 MB (31%). Most users install Entwine
   as a precompiled npm binary, so they would all pay this for one page shell.
2. Making Flowview the default or only renderer raises the source-build MSRV
   from 1.85 to 1.91.
3. The gain is readability of a 117-line shell that rarely changes, and it
   comes with slightly more code, not less.

ADOPT would trade a permanent cost to every user for a small maintenance gain.
REJECT would throw away integration work that is now cheap to keep: the
dependency is from the registry, the feature is isolated, and the permanent
baseline makes any drift between the two renderers visible.

Reconsider adoption only if one of the following happens:

- Flowview offers a static rendering path without Oxc, or the binary cost
  falls below about 250 KB.
- Entwine's page shell grows in size or changes often enough, for example
  through user-facing layout work, that a template language clearly pays off.

If neither happens within the next two minor releases, remove the experiment
(option C). Doing so means deleting the dependency, `page.flow`, the adapter,
the renderer option and the experimental CI job, and keeping
`tests/baseline/`.

### Not changed

The built-in renderer is still the default. `RendererKind`, the feature gate,
the Rust 1.85 MSRV, the production deployment and Flowview itself were not
changed.
