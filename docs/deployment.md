---
type: guide
---

# Deployment

Entwine produces portable static output. Publishing is a separate concern, and
Entwine does not host anything.

```text
entwine init    → scaffold recommended project knowledge
entwine dev     → read it locally
entwine check   → validate it
entwine setup   → automate validation and publishing, once
entwine build   → portable dist/ for any other hosting
```

`init` is about knowledge structure. `setup` is about repository automation.
After `setup`, the provider's CI owns publishing; Entwine has no `publish`
command and stores no credentials. See
[the decision](decisions/provider-owned-publishing.md) and the
[generated CI contract](specs/generated-ci.md).

## Portable output: `entwine build`

`entwine build` writes `dist/` containing plain HTML, CSS, and copied assets with
relative links and no provider-specific code. Serve it with nginx, Caddy, S3 or
MinIO, Cloudflare Pages, Netlify, internal or air-gapped infrastructure, or
anything that serves static files, including below a subpath. Ignoring
`entwine setup` entirely is a first-class choice.

## `entwine setup`

```sh
entwine setup --dry-run   # show the plan; change nothing
entwine setup             # write the configuration
```

Setup inspects local Git remotes without any network call. It prefers the remote
of the current branch, then `origin`, then the remaining remotes in sorted order,
and uses the first one it recognizes.

| Remote host | Provider | Native publishing |
| --- | --- | --- |
| `github.com` | GitHub | GitHub Pages via GitHub Actions |
| `gitlab.com`, or a host starting with `gitlab.` | GitLab | GitLab Pages via GitLab CI/CD |
| `bitbucket.org` | Bitbucket Cloud | Bitbucket static website hosting |
| anything else | Unknown | portable `dist/` guidance |

Both HTTPS and SSH (`git@host:owner/repo.git`) remotes work; credentials embedded
in a URL are discarded. Other self-hosted servers are never guessed. Choose one
explicitly with `--provider github|gitlab|bitbucket`.

In a monorepo, run `entwine setup path/to/project`; paths and commands are
prefixed accordingly. Directory names with characters that cannot be safely
embedded in CI configuration are refused.

All generated pipelines do the same thing:

- A pull or merge request changing repository files runs `entwine check`. Validation errors
  fail it. Missing recommended knowledge areas stay warnings.
- The default branch, on repository changes, runs
  `entwine check`, then `entwine build`, then publishes that same built artifact.
- Pull requests are never published.

Generated files install `@entwine/cli` pinned to the version of the CLI that wrote
them, through a single clearly marked install line and version variable. If you
use a different installation path, such as a release binary, change that line.
Generated setup pins the CLI version for reproducible builds. See
[current state](state.md#release-status) for which versions are published.

### GitHub Pages

Writes `.github/workflows/entwine.yml`. It uses the official Pages actions
(`actions/upload-pages-artifact` and `actions/deploy-pages`) with major versions
pinned. Workflow permissions are `contents: read`; only the deploy job receives
`pages: write` and `id-token: write`. The default branch is read from the
repository rather than hard-coded.

One-time step: in the repository, open Settings → Pages and set Source to
"GitHub Actions". Availability for private repositories depends on your plan.

### GitLab Pages

Writes `.gitlab/ci/entwine.yml` and adds an `include:` to `.gitlab-ci.yml`. If
there is no `.gitlab-ci.yml`, one is created. If it exists and defines `include`,
`stages`, or `workflow`, or starts with `---`, it is left untouched and setup tells
you the two lines to add. The pipeline uses `$CI_DEFAULT_BRANCH`, so no branch
name is embedded, and the `pages:publish` keyword (GitLab 17.10 or later). It
works on GitLab.com and Self-Managed; set the `ENTWINE_IMAGE` CI/CD variable if
your runners cannot pull `node:22`. Self-Managed hosts whose name does not start
with `gitlab.` need `--provider gitlab`.

### Bitbucket Cloud

Bitbucket's native static hosting has constraints that Entwine does not hide:

- It serves one site per workspace, from the main branch of a repository named
  `<workspace>.bitbucket.io`, at `https://<workspace>.bitbucket.io`.
- It is public even if the repository is private, and pages are cached for about
  15 minutes.
- Pipelines cannot publish to it without a credential for that repository.

So `entwine setup` writes `bitbucket-pipelines.yml` (validate on pull requests; on
the default branch, build and push `dist/` into a folder named after the
repository inside the site repository) and reports the one-time manual steps:

1. Create the `<workspace>.bitbucket.io` repository if it does not exist.
2. Create a repository access token on it with write access.
3. Add it to this repository's Pipelines settings as the secured variable
   `ENTWINE_SITE_TOKEN`.
4. Enable Pipelines.

The token exists only as a Bitbucket secured variable. It is never written to a
file, printed, or placed in a URL. If `bitbucket-pipelines.yml` already exists,
Bitbucket has no include mechanism, so it is never edited: the pipelines are
written to `.entwine/bitbucket-pipelines.snippet.yml` for you to merge.

### Unknown providers

```text
✓ Git repository detected
⚠ No supported native publishing provider detected.

Entwine output remains portable:

    entwine build

Publish dist/ using your existing static hosting pipeline.
```

`setup` does not fail and does not push you toward any particular host.

## Safety and idempotency

- Existing files are never overwritten. A file at a generated path that was not
  created by Entwine is reported and left alone.
- Files created by `setup` carry an `entwine-managed` marker. Re-running `setup`
  reports `✓ already configured` and changes nothing, even after the pinned version
  moved on.
- Only `.gitlab-ci.yml` can be modified, and only to prepend an `include`, when it
  has no `include`, `stages`, or `workflow`. Nothing else shared is edited.
- Setup never writes through symbolic links, rejects unsafe project paths, embeds
  no credentials, and executes no repository-provided strings.
- Generated workflows use least privilege and never deploy pull requests.

## Change or remove the setup

Delete the generated file or files (and the `include` lines in `.gitlab-ci.yml`)
to remove it; `entwine build` is unaffected. To regenerate after an upgrade,
delete the file and run `entwine setup` again. Edit the version variable to move
to another Entwine release.

## Official showcase

Entwine's own site is deployed to Cloudflare Pages by a workflow in the Entwine
repository: the Astro website at `/`, these docs at `/docs/`, and the kitchen-sink
demo at `/demo/`. That is the project's own choice. It is unrelated to what
`entwine setup` generates for your repository. See [the architecture](architecture.md) for
the output model.

Generated validation also runs for repository-file changes outside `docs/`, because
those files can be targets of repository references. Publishing remains restricted
to the default branch.
