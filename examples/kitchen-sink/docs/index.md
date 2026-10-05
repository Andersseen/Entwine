# Harbor

Harbor is a small, fictional service for teams to catalogue shared datasets. This
site is the documentation for that imaginary project, and it is a worked example
of Entwine.

## How this site was made

- Every page is a plain Markdown file under `docs/`. There is no documentation
  framework, theme, or configuration file in this project.
- Entwine compiled those files into this static site. Folders became navigation
  and files became routes.
- Ordinary Markdown links create relationships between pages. Look for the
  **Referenced by** list at the bottom of a page: backlinks are derived
  automatically.
- Open **Project graph** in the sidebar to see the same relationships drawn by
  Entwine as a linked SVG.
- Broken links, bad anchors, and route collisions fail the build instead of
  shipping.

## Start here

- Understand the [architecture](architecture.md).
- Review the [current state](state.md) and [roadmap](roadmap.md).
- Read the [authentication](specs/authentication.md) and [search](specs/search.md) specifications.
- Learn why we chose [static output](decisions/static-output.md).

## Local workflow

```sh
entwine check
entwine dev
```

Your Markdown remains the source of truth. See [CommonMark](https://commonmark.org)
for the underlying Markdown syntax.
