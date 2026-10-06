# Generated CI contract

What every pipeline written by `entwine setup` guarantees, whatever the provider.
See [deployment](../deployment.md) for the provider details.

## Behavior

- A pull or merge request changing repository files runs `entwine check` and fails on
  Entwine validation errors only. Recommended-knowledge gaps are warnings.
- A repository change on the default branch runs `entwine check`, then
  `entwine build`, then publishes the artifact that build produced, without
  rebuilding differently at deploy time.
- Unrelated source changes do not trigger documentation builds.
- Pull and merge requests are never published.

## Safety

- Least privilege: on GitHub only the deploy job can write Pages or request an
  OIDC token.
- No credential appears in any generated file. Bitbucket's token is a secured
  provider variable.
- Generation is deterministic and idempotent; existing files are never replaced.
- The default branch comes from the provider where it exposes one.

## Non-goals

No `entwine publish`, no hosted service, and no code-to-documentation drift
detection. These are tracked in the [roadmap](../roadmap.md).

Repository references may point outside `docs/`, so generated triggers cover the
whole repository. PR/MR events validate; only default-branch events publish.
