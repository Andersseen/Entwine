---
type: decision
status: accepted
---

# Read-only first

## Context

Harbor must be useful before it has users, sessions, or permissions.

## Decision

Ship read-only dataset records first and add writes later.

## Why

Reads can be public and reproducible. Writes need the
[authentication specification](../specs/authentication.md) first.

## Consequences

The [data model](../design/data-model.md) has no mutation paths yet, and the
[roadmap](../roadmap.md) sequences writes after authentication.

## Alternatives

Building authentication and writes together was rejected as a larger first release.
