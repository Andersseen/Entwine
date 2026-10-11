# @entwine/mcp

A local [Model Context Protocol](https://modelcontextprotocol.io) server (stdio) that
lets coding agents read an Entwine-enabled repository's project knowledge, validate
it, and preview setup, without pasting documentation into the conversation.

It is a thin adapter over the Entwine CLI: every answer comes from
`entwine context --json`, `entwine check` or a `--dry-run` command. It has no
Markdown parser, graph engine, network access or write path of its own.

> Pre-1.0 and experimental. Check `npm view @entwine/mcp versions` before relying
> on a version; an unpublished version cannot be installed from npm.

## Run

```sh
npx --yes @entwine/mcp --root /path/to/repository
```

Project root, in order of precedence: `--root <dir>`, `ENTWINE_PROJECT_ROOT`, the
working directory. The root is canonicalized and must be an existing directory.

Environment: `ENTWINE_BIN` (use this entwine executable or `.js` launcher instead of
the bundled `@entwine/cli`), `ENTWINE_MCP_TIMEOUT_MS` (per-call CLI timeout, default 60000).

Register in Codex: `codex mcp add entwine -- npx --yes @entwine/mcp --root .`
Register in Claude Code: `claude mcp add entwine -- npx --yes @entwine/mcp --root .`

## Surface

Tools (all read-only; previews are `--dry-run`): `entwine_project_overview`,
`entwine_list_documents` (role/artifact/query filters, `limit`/`offset`),
`entwine_read_document` (bounded slice, `offset`/`max_chars`), `entwine_related_documents`,
`entwine_check`, `entwine_init_preview`, `entwine_setup_preview`.

Resources: `entwine://project/summary` and `entwine://document/{id}` (id percent-encoded,
e.g. `entwine://document/decisions%2Fstatic-output.md`).

Prompts: `entwine_adopt`, `entwine_maintain_knowledge`, `entwine_prepare_publishing`.

## Boundaries

- Documents are addressed only by ids present in the Entwine context; no tool takes a file path.
- No tool writes files, runs arbitrary commands, deploys, or changes CI. To apply
  `init` or `setup`, an agent runs the CLI through its host's normal approval flow.
- The CLI is run without a shell, with an argument array, a timeout, and cancellation.
- stdout carries protocol messages only; diagnostics go to stderr.

## Compatibility

`@entwine/mcp` is released in lockstep with `@entwine/cli` and depends on the exact
same version. It supports context `schema_version` `"0.4"` and rejects any other with
an actionable error. With `ENTWINE_BIN`, the schema check still applies.
