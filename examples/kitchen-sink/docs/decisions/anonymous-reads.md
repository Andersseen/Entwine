---
type: decision
status: accepted
---

# Anonymous public reads

## Context

Teams need to look up shared datasets without an account.

## Decision

Public catalogue reads never require a session.

## Why

Anonymous reads stay cacheable and reproducible, as the
[architecture](../architecture.md#request-lifecycle) requires.

## Consequences

An expired session must never block public search. See the failure behavior in the
[authentication specification](../specs/authentication.md#failure-behavior).

## Alternatives

Requiring sign-in for every read was rejected as unnecessary friction.
