---
title: Harbor architecture
---

# Architecture

Harbor separates catalogue storage from its public read interface. See the
[home page](index.md) for the project overview and the [data model](design/data-model.md)
for how records are shaped.

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
> This follows from [anonymous public reads](decisions/anonymous-reads.md).

## Deployment

Documentation follows the [static output decision](decisions/static-output.md).
What is deployed today is recorded in the [current state](state.md).
