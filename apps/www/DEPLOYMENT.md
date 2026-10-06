# Cloudflare Pages deployment (Entwine's own showcase)

This is how the Entwine project publishes its own website, docs, and demo. It is
not what `entwine setup` generates for users, who get provider-native publishing
(GitHub Pages, GitLab Pages, Bitbucket); see `docs/deployment.md`.

One Pages project serves three things. Two of them are Entwine output.

```text
deployment/
├── index.html, _astro/, logo.png   Astro website (apps/www)            → /
├── docs/                           `entwine build .`                   → /docs/
│   ├── architecture/, state/, roadmap/
│   └── __entwine/ (style.css, graph/)
└── demo/                           `entwine build examples/kitchen-sink` → /demo/
    ├── architecture/, specs/, decisions/
    └── __entwine/ (style.css, graph/)
```

`/docs/` and `/demo/` are not recreated in Astro. They are Entwine's own relative-URL
output mounted below a subpath; Entwine contains no Cloudflare-specific logic.

## Build locally

```sh
pnpm install --frozen-lockfile
pnpm build:showcase
```

This builds Entwine, validates and builds both documentation sets, builds Astro,
composes `deployment/` (`tooling/compose-showcase.ts`, which fails if an input
is missing), and verifies every relative link, asset, and fragment under `/docs/`
and `/demo/` (`tooling/verify-showcase.ts`). `deployment/` is exactly what is
uploaded. Preview it with any static server, e.g. `npx serve deployment`.

`pnpm dev:www` (Astro dev) only renders the landing page itself; it proxies `/docs/`
and `/demo/` from `deployment/` after a `pnpm build:showcase`.

## Workflow

`Deploy showcase to Cloudflare Pages` runs on every push to `main` and on manual
`workflow_dispatch` (run it on `main`). The `validate` job runs `pnpm check`:
formatting, lint, Clippy, TypeScript/Astro checks, Rust tests, and
`pnpm build:showcase`. It uploads `deployment/` as an artifact. The `deploy` job
downloads that exact artifact and publishes it with Wrangler; nothing is rebuilt.
Pull requests run the CI workflow, which builds and verifies the same artifact
but never deploys. Preview deployments are not configured.

## One-time setup

1. Create a Cloudflare Pages **Direct Upload** project with production branch
   `main`. Do not enable a separate automatic Git build.
2. Create an API token with **Account → Cloudflare Pages → Edit**, scoped to the
   account hosting the project.
3. In GitHub repository Settings → Secrets and variables → Actions, add:
   - Secret `CLOUDFLARE_API_TOKEN`: the token.
   - Secret `CLOUDFLARE_ACCOUNT_ID`: the Cloudflare account ID.
   - Variable `CLOUDFLARE_PAGES_PROJECT_NAME`: the existing Pages project name.
4. Merge into `main`, or run the workflow manually on `main`.

No credentials belong in the repository. The workflow does not create the
Cloudflare project or configure a custom domain. Pages redirects `/docs` to
`/docs/`, which the relative links rely on.

See [Cloudflare's Direct Upload CI guide](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/).
