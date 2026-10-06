---
type: spec
status: implemented
---

# Catalogue export

An existing capability: a deterministic export of dataset records. It is
described in the [current state](../state.md#available-today).

## Behavior

Records are written in `id` order, so identical catalogues always produce
identical files. The shape follows the [data model](../design/data-model.md).

## Failure behavior

An empty catalogue exports an empty list, never an error.
