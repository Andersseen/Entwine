# Catalogue API instructions

These instructions apply to everything under `services/catalogue-api/`.
They extend the [project instructions](../../AGENTS.md).

- Responses follow the [catalogue export spec](../../docs/specs/catalogue-export.md).
- Search behavior is defined by the [search spec](../../docs/specs/search.md); do
  not invent ranking rules in handlers.
- Keep the [data model](../../docs/design/data-model.md) and migrations in step.
- Public reads are anonymous by [decision](../../docs/decisions/anonymous-reads.md).
