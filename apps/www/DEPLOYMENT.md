# Cloudflare Pages deployment

The `Deploy www to Cloudflare Pages` workflow publishes the static Astro site
after `pnpm check` passes: formatting, lint, Rust Clippy, TypeScript/Astro checks,
Rust tests, the website build, and Entwine documentation validation.
The deployment job downloads the exact build artifact from the validation job.
Pull requests run the existing CI workflow without publishing.

## One-time setup

1. Create a Cloudflare Pages **Direct Upload** project with production branch
   `main`. Do not enable a separate automatic Git build for this workflow.
2. Create an API token with **Account → Cloudflare Pages → Edit**, scoped to the
   account hosting the project.
3. In GitHub repository Settings → Secrets and variables → Actions, add:
   - Secret `CLOUDFLARE_API_TOKEN`: the token.
   - Secret `CLOUDFLARE_ACCOUNT_ID`: the Cloudflare account ID.
   - Variable `CLOUDFLARE_PAGES_PROJECT_NAME`: the existing Pages project name.
4. Merge into `main`, or run the workflow manually on `main`.

No credentials belong in the repository. The workflow deploys `apps/www/dist`,
not the documentation compiler's root `dist`. It does not create the Cloudflare
project or configure a custom domain.

See [Cloudflare's Direct Upload CI guide](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/).
