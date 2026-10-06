---
type: architecture
---

# Data model

This document uses a custom filename. Its `type: architecture` metadata is enough
for Entwine to treat it as architecture alongside [Harbor architecture](../architecture.md).

## Records

A dataset record has a stable `id`, a `name`, and a `description`. Records are
read-only for now, as decided in [read-only first](../decisions/read-only-first.md).

## Exports

The [catalogue export](../specs/catalogue-export.md) serializes records in `id`
order so output is reproducible.
