---
title: Authentication
type: spec
status: planned
---

# Authentication

This is a proposal for Harbor, not an Entwine capability. Return to
[architecture](../architecture.md) for the system context.

## Session boundary

A future session identifies a writer. Public catalogue reads remain anonymous, as
decided in [anonymous public reads](../decisions/anonymous-reads.md).

## Failure behavior

An expired session must reject mutations. It must not prevent public
[search](search.md#query-contract).

```text
expired session → reject write → preserve catalogue
```

Scheduled in the [roadmap](../roadmap.md).
