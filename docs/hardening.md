# Standalone hardening evidence

This milestone strengthens Entwine itself. It adds no AI, translation, agent,
MCP, or external-project integration. The Knowledge Convention stays recommended,
not required. Presence is not quality.

## Distribution truth

On 6 October 2026, public `v0.1.0` was verified non-draft with four native archives
and SHA256SUMS. Archives were approximately 0.99–1.25 MB. All five npm packages
were verified at 0.1.0: `@entwine/cli`, Linux x64, macOS arm64/x64, and Windows x64.
A clean macOS arm64 consumer installed the real registry launcher and passed
version, init, check, build, and setup fallback. The macOS arm64 native archive
also passed SHA-256 verification and the same first-run build path.

The package remains 0.1.0 in this branch. These changes are unreleased; the next
package version belongs to release-please and Conventional Commits. Context
schema versioning is separate: [0.3](specs/context-schema.md) adds explicit
anchors and repository references, and preserves full original source including
frontmatter.

## Compatibility decisions

- Root `index.md` wins; root `README.md` falls back to `/` and project role when
  the index is absent. Both can coexist. Nested READMEs remain ordinary pages.
- Documentation directories with documents receive generated navigation-only
  indexes. Existing document routes always win. Generated pages are not knowledge
  documents and create no invented relationships.
- Existing files inside the repository but outside `docs/` are distinct repository
  references retaining path, source, line, and original destination. Missing files,
  traversal outside the repository, and symlink crossings remain errors.
- Local Git metadata supplies source URLs for GitHub, GitLab, and Bitbucket Cloud,
  using an immutable local commit and encoded URL segments. Credentials are
  discarded. Unknown providers and files absent at that commit render labeled
  repository paths. No Git commands enter the core and no build needs the network.
- Only empty `<a id="safe-id"></a>` or `<a name="safe-id"></a>` declarations with
  one quoted ASCII identifier are converted into generated spans. Extra attributes,
  executable HTML, entities, and nonempty anchors remain escaped. Explicit IDs
  reserve heading slugs; duplicates and the renderer’s reserved `main` fail.
- Missing-fragment diagnostics identify the existing document and missing anchor,
  with source and one-based line. Repository fragments are retained but not
  validated as documentation headings.

## Real repository evidence

Inputs were the local ForgeCMS, Flowview, and Agentyx working trees, read-only.
These are validation consumers, not integrations. Counts describe those snapshots;
CI uses small distilled regression fixtures instead of external repositories.

| Consumer | Documents | Relationships | Repository references | HTML pages | Result | Warnings |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| ForgeCMS | 89 | 176 | 18 | 94 | 3 valid missing-path errors | 27 |
| Flowview | 2 | 1 | 0 | 5 | Compile + isolated-copy build passed | 1 |
| Agentyx | 10 | 15 | 7 | 14 | Compile + isolated-copy build passed | 2 |

ForgeCMS’s remaining errors are `DEMO-FINDINGS.md` lines 3, 77, and 114: its
`apps/demo-aesthetics` path and two files below it no longer exist. These are
repository content problems; Entwine correctly rejects them. Its raw anchors,
README entrypoint, directory links, and eighteen existing repository references
now resolve. ForgeCMS has project, architecture, state, roadmap, and spec coverage,
but no decision documents. Flowview and Agentyx have custom knowledge without the
recommended roles; normal check stays permissive. Their missing coverage is a
convention finding, not a compiler failure.

The source-line index fixes an observed pathological repeated scan of ForgeCMS’s
large state document. After the fix its release-mode compilation was about 41 ms
and graph rendering about 1 ms on the local machine.

Repository directories and images outside `docs/` remain intentional limitations:
reference a regular file or keep published assets under `docs/`. Arbitrary HTML
remains escaped. These are future evidence-driven candidates, not silently relaxed
validation.

## Graph and reading experience

Small graphs retain the full radial view. More than twelve documents use role
bands. More than fifty documents or two hundred edges prioritize a complete open
role-grouped index, with an optional full SVG. Edges always come from Markdown.
Relationship text remains complete and supports normal browser Find.

Pages show outgoing References, Referenced by, and source paths/links. Generated
navigation, Knowledge, graph, assets, and fragments remain relative under `/docs/`
and `/demo/`. Long sibling navigation lists show twenty entries plus the current
page and ancestors, with an explicit route to all documents in Knowledge.

The neutral renderer retains light/dark CSS, native mobile disclosures, native
landmarks, skip links, current-page markers, SVG descriptions, and visible focus.
Main targets accept skip-link focus; code blocks and tables accept keyboard scrolling.
Large References/Referenced by lists use native disclosures without losing data.
Muted text contrast and long-title wrapping were improved. The demo now includes
repository references, generated directory indexes, and a small local SVG asset.
The homepage exposes installation, documentation, live demo, Knowledge, graph,
canonical/social metadata, native releases, and checksums.

## Performance baseline

Synthetic documents contain about 1.7 KB of paragraphs, headings, code, and a
chain link. Release builds, one local measurement per size; process timings include
startup and Git detection. RSS is macOS `/usr/bin/time -l`, not a portable memory
estimate. Measurements are a baseline, not a performance guarantee.

| Documents | Check ms | Build ms | Compile ms | Graph ms | Render ms | Output MB | Build RSS MB |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 14.34 | 11.71 | 0.394 | 0.067 | 0.155 | 0.069 | 8.4 |
| 100 | 23.34 | 34.83 | 3.219 | 0.424 | 1.742 | 0.706 | 11.8 |
| 500 | 57.43 | 131.08 | 23.465 | 2.187 | 8.954 | 3.496 | 25.9 |
| 1000 | 518.49 | 282.27 | 103.911 | 4.998 | 19.378 | 6.992 | 44.3 |

Before bounding duplicated navigation, the 1,000-document output was 110.46 MB,
with about 432 ms rendering and 150.8 MB peak build RSS. The measured fix reduced
those to 6.99 MB, 19.4 ms, and 44.3 MB. Check timing varies with filesystem and
concurrent work; no incremental compiler or client search was justified.

Reproduce compilation/projection statistics with:

```sh
cargo run --release -p entwine-engine --example audit -- /path/to/project
```

This helper is read-only and can report diagnostics/counts even when publication
is correctly blocked by invalid content.

## Provider and release evidence

| Provider | Generation | YAML + contract tests | Real generated pipeline | Real provider-native publish |
| --- | --- | --- | --- | --- |
| GitHub | Yes | Yes | No | No |
| GitLab | Yes | Yes | No | No |
| Bitbucket Cloud | Yes | Yes | No | No |

Entwine’s own main CI, release, and Cloudflare deployment were verified successful
at commit `51644a0`. That is not evidence of a consumer GitHub Pages deployment.
No dedicated hosted consumer repository or GitLab/Bitbucket credentials were used.
GitLab’s configuration requires 17.10+; Bitbucket requires its workspace website
repository and a secured write token. No credentials are generated or printed.

Existing gates remain. New CI adds macOS/Windows regression tests and real registry
consumer smoke. Release hardening adds early version/lock checks, exact-ref quality
gates, explicit retry of an existing draft tag, and a registry smoke before release
visibility. Already published npm versions are skipped; assets use clobber uploads.
Release-please remains authoritative. The existing Cargo.lock sync is retained.
Generated provider triggers cover repository changes, including referenced files
outside docs/. Default-branch publication restrictions remain enforced.
Bitbucket validates provider-derived workspace/repository slugs before filesystem
operations and removes its temporary askpass file.

The open upload-artifact major upgrade was reviewed against official compatibility
notes. It was not blindly merged. Hosted-runner validation of changed workflows
and public deployment of this branch must be recorded separately from local gates.

## Validation record

The 1/5/12/20/50/100-document browser matrix covered documentation, Knowledge,
and graph at 390px and 1440px in light and dark modes: 72 page variants. Axe WCAG
A/AA checks found no violations after fixing scrollable-code focus. The marketing
homepage also passed axe. No page-wide horizontal overflow occurred. Keyboard
checks verified skip-link focus on main in all four viewport/color combinations;
mobile disclosures were opened in the browser. This does not replace a human
screen-reader audit.

Executed local gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
pnpm lint
pnpm format:check
pnpm typecheck
pnpm build:showcase
pnpm smoke:setup
pnpm smoke:npm
entwine check .
entwine check --strict-knowledge .
entwine build .
entwine check examples/kitchen-sink
entwine build examples/kitchen-sink
```

The npm packing smoke uses an isolated cache and local host-platform tarballs
in offline mode; the separate registry smoke tests the real published package.
Dev-server integration covers repeated edits, invalid→valid recovery, repository
file removal/restoration, new nested directories, rename/deletion, asset changes,
and preservation of the last valid output. Security regressions cover traversal,
encoded paths, in-repository symlinks, unsafe anchors, duplicate/reserved IDs,
unsafe source prefixes, credential-bearing/unknown remotes, and case collisions.
Exact goldens cover site, directory page, Knowledge, graph, context, and provider
YAML. Only package version text is normalized.

macOS arm64 is the local execution platform. Linux/macOS/Windows hosted regression
results and this branch’s public deployment are recorded after remote CI runs.
