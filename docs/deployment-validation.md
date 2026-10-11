---
type: guide
---

# Deployment validation

What is verified about `entwine setup` output and `entwine build` under
GitHub Pages, GitLab Pages, and Bitbucket Pipelines, and what is not. Verified
10–11 October 2026 against commit `314bc95`.

## Evidence matrix

| Contract                                                                            | Evidence                                                   | Level            |
| ----------------------------------------------------------------------------------- | ---------------------------------------------------------- | ---------------- |
| Output links, assets, anchors and navigation resolve at `/`, `/repo/`, nested paths | Static-host simulator crawl of real `entwine build` output | Local simulation |
| Encoded names (space, `+`, `&`, non-ASCII), nested routes, case-exact paths         | Same crawl; exact-case, byte-exact lookups                 | Local simulation |
| Repository references are provider URLs, independent of the Pages base             | Built in a Git repository with a GitLab remote             | Local simulation |
| Identical output regardless of checkout location                                    | Byte comparison of two builds                              | Local            |
| Empty or missing `docs/` fails in `check` with an actionable error                  | CLI exit status and message                                | Local            |
| GitHub workflow: triggers, least privilege, job graph, upload path = real output    | YAML parsed; `entwine` commands from the file are executed | Structural       |
| GitLab pipeline: jobs, stages, `extends`, `needs`, `pages.publish` = artifact path  | YAML parsed; commands executed; artifact crawled           | Structural       |
| Bitbucket pipeline: step keys, branch-only publish, no PR publish                   | YAML parsed against the step schema subset Entwine emits   | Structural       |
| Bitbucket publish script                                                            | Run for real against a local bare Git repository           | Local simulation |
| Action tags `checkout@v7`, `setup-node@v7`, Pages actions `@v5` exist              | `gh api repos/<action>/git/ref/tags/<tag>`, read-only      | Remote, read-only |
| GitHub Pages, GitLab Pages, Bitbucket hosting actually serve a generated site       | **Not performed**                                          | Not verified     |

No provider executed a generated pipeline. No GitLab or Bitbucket credential
existed in this environment, and creating a disposable GitHub repository or
changing Pages settings was not authorized. Claims above are local; none is a
live deployment.

## Verified behavior

- Every generated link is relative, so one build serves at the root or any
  subpath: a GitHub project site (`/repo/`), a GitLab project or nested group
  (`/group/sub/project/`), a Bitbucket folder (`/<repository>/`), or a unique
  domain. No `base` setting exists or is needed.
- Directory links always end in `/`, so no host redirect is relied on.
- Names that cannot be a URL path segment (`?`, `%`, `#`) are rejected by
  `entwine check` and never reach the pipeline. A link whose case differs from
  the file is a broken link, because Linux hosts are case-sensitive.
- Monorepo layouts (`entwine setup apps/site`) upload or publish
  `apps/site/dist`, the directory `entwine build apps/site` creates.
- Empty sites are unsupported by design: `docs/index.md` is required.

## Confirmed defect, fixed

The Bitbucket publish script rejected repository slugs containing a period.
[Atlassian documents](https://support.atlassian.com/bitbucket-cloud/kb/what-is-a-repository-slug/)
periods as valid in slugs, so `my.docs` failed every publish. Slugs may now
contain periods, but not begin with one (that could name `.git` in the site
repository). Workspace IDs remain strict.

## Limitations

- Bitbucket publishing needs `<workspace>.bitbucket.io` and `ENTWINE_SITE_TOKEN`
  as described in [deployment](deployment.md); the script was never run
  against bitbucket.org.
- GitLab `pages.publish` needs GitLab 17.10 or later; not run against GitLab.
- Generated triggers cover the whole repository, because repository references
  may change anywhere; documentation-only path filtering is not offered.
- A leading-period or hyphen Bitbucket slug is refused.

## Reproduce

```sh
cargo test -p entwine-cli --test deployment --test workflows
pnpm smoke:setup
pnpm build:showcase   # relative references under /docs/ and /demo/
```

To obtain live evidence, in disposable repositories you own: run `entwine setup`,
push to the default branch, wait for the pipeline, then fetch the published URL
and run a crawl such as the one in `crates/entwine-cli/tests/deployment.rs`
against it. Record the run URL and result here.
