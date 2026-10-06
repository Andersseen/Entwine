# Current state

The catalogue supports read-only dataset records. The [architecture](architecture.md)
describes the boundaries that are already implemented.

## Available today

- Dataset names and descriptions through the read interface.
- Static documentation, published as decided in [static output](decisions/static-output.md).
- A deterministic [catalogue export](specs/catalogue-export.md).

## In progress

Nothing is partially implemented. Search is only specified; see the
[search contract](specs/search.md).

## Known limitations

Write access and full-text search are not shipped. They are planned in the
[roadmap](roadmap.md). Day-to-day handling is in the [operations runbook](operations/runbook.md).
