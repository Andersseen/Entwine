# @entwine/cli

[Entwine](https://github.com/Andersseen/Entwine) — project knowledge, connected.
A repository-native compiler that turns Markdown under `docs/` into static
documentation, backlinks, a project graph, validation, and structured context.

```sh
npm install --global @entwine/cli
entwine check
entwine dev
```

This package is a launcher for the native Rust binary, delivered through the
platform packages `@entwine/cli-darwin-arm64`, `-darwin-x64`, `-linux-x64`, and
`-win32-x64`. There is no postinstall script and no network access at install
time beyond your package manager. Experimental, pre-1.0. MIT licensed.
