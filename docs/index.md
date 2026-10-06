---
type: project
---

# Entwine

Project knowledge, connected.

Entwine turns the Markdown already in a repository into one shared source of
project knowledge for humans and agents.

```text
            Markdown
               │
         Knowledge model
        /      |      \
      Docs   Graph   Context
    (humans)  (both)  (agents)
```

Docs, the project graph, and structured context are projections of the same
parsed knowledge. Markdown remains the source of truth.

## Where to start

- [Architecture](architecture.md): how Entwine is structured today.
- [Current state](state.md): what is implemented and what is not.
- [Roadmap](roadmap.md): where it is headed.
- [Knowledge convention](convention.md): the recommended, optional structure
  for durable project knowledge.
- [Hardening evidence](hardening.md): real consumer findings, measurements, and verification.
- [Deployment](deployment.md): `entwine setup` and provider-native publishing.

## Decisions and specifications

- [Derive roles; keep author metadata open](decisions/derived-roles.md)
- [Provider CI owns publishing](decisions/provider-owned-publishing.md)
- [Context schema](specs/context-schema.md)
- [Generated CI contract](specs/generated-ci.md)

This documentation is itself compiled by Entwine and follows its own
convention. Entwine is experimental and pre-stable.
