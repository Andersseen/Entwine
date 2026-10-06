---
type: decision
status: accepted
---

# Static output

## Context

Harbor's documentation should be cheap to host and easy to move.

## Decision

Publish documentation as HTML and CSS.

## Why

This keeps production free of a dedicated documentation runtime.

## Consequences

- Deploy to any static file host.
- Rebuild after Markdown changes.
- Preserve links from the [architecture](../architecture.md#deployment).

## Alternatives

A hosted documentation platform was rejected to avoid a runtime dependency.

The [project home](../index.md) is the starting point for new contributors.
