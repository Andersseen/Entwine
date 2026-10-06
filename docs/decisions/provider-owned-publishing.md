# Provider CI owns publishing

## Context

Users host repositories on GitHub, GitLab, Bitbucket, or elsewhere. Each provider
has a native static hosting mechanism and its own credentials. Entwine's own
showcase uses Cloudflare Pages, which suits that project but not every consumer.

## Decision

`entwine setup` generates provider-native CI once, and the provider's CI owns
validation and publishing from then on. There is no `entwine publish` command.
`entwine build` stays portable. Entwine's own Cloudflare deployment is kept
separate from what users receive.

## Why

Owning publishing would make Entwine responsible for GitHub, GitLab, and
Bitbucket authentication, hosting APIs, and credential storage. Generated CI keeps
secrets in the provider, is reproducible, and works for air-gapped or custom
hosting through plain `dist/`.

## Consequences

- Generated pipelines are ordinary, editable files, and setup never overwrites
  existing CI.
- Each provider's constraints are surfaced, not hidden. Bitbucket needs a manual
  one-time step. See [deployment](../deployment.md).
- Generated YAML must be kept current with provider syntax.

## Alternatives

A hosted or Cloudflare-by-default path was rejected as not provider-native. An
`entwine publish` command was deferred because of the credential surface.
