# Derive roles; keep author metadata open

## Context

Entwine already accepted open-ended `type`, `status`, and `title` metadata. The
[convention](../convention.md) needs a small set of meanings (architecture, state,
roadmap, decision, spec) that tools can rely on. Real repositories use arbitrary
filenames and arbitrary `type` values.

## Decision

Add a derived `KnowledgeRole`, separate from the author's `type`. A recognized
`type` wins, then the canonical path, then `other`. The raw `type` is kept
unchanged. Conflicts warn and never fail.

## Why

A closed enum in place of `type` would reject or rewrite author data and break
existing repositories. Deriving the role in `entwine-core` gives every projection,
including [structured context](../specs/context-schema.md), the same answer
without a second parser.

## Consequences

- Repositories adopt roles by metadata without renaming files.
- Context consumers can branch on `role` and still read the author's `type`.
- Adding a role later changes the role set, which is versioned with the context
  schema.

## Alternatives

Making `type` a closed set was rejected for the reasons above. Path-only
classification was rejected because it forces renames. A configuration file for
custom mappings was deferred until real users need it.
