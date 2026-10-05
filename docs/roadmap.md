# Roadmap

This page describes future intent. The [current state](state.md) lists what works.

## Next milestone

Validate Entwine on repositories that are not its own. The 0.2 checks use
generated trees and one fictional example; real documentation will find the
remaining gaps. Likely work, in order of evidence:

- Fix path, link, or Markdown issues found by running `entwine check` on
  established open-source `docs/` directories, including reference-style links
  and nested READMEs.
- Improve graph readability beyond roughly 50 documents, where the fixed radial
  layout produces a wide canvas that the text relationship list serves better.
- Decide whether `graph` should differ from `build`, based on how people use it.

## Later considerations

Consider small opt-in configuration only when real users demonstrate a need.
Evaluate search, broader distribution such as package managers or crates.io, and
context schema evolution separately. Preserve the
[canonical model boundary](architecture.md#canonical-representation); separating
compiled HTML from the canonical document is worth revisiting only if a second
renderer needs it.

No integration or AI roadmap is promised.
