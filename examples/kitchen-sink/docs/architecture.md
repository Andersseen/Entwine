---
title: Harbor architecture
type: guide
status: implemented
---
# Architecture

Harbor separates catalogue storage from its public read interface. See the
[home page](index.md) for the product thesis.

## Boundaries

| Component | Responsibility |
| --- | --- |
| Catalogue | Own dataset names and descriptions |
| Read interface | Return stable public records |
| Documentation | Explain decisions and contracts |

### Request lifecycle

1. Validate a request.
2. Apply the [authentication contract](specs/authentication.md#session-boundary).
3. Query the catalogue using the [search contract](specs/search.md).

> Keep public reads reproducible. Do not infer permissions from UI state.

## Deployment

Documentation follows the [static output decision](decisions/static-output.md).
