---
type: guide
---

# Agent integration

How an agent working in **another** repository can discover, understand, validate
and adopt Entwine. Two independent pieces exist; use either or both.

| Piece | What it is | Needs |
| --- | --- | --- |
| **Skills** (`skills/`) | Markdown instructions an agent loads on demand: `entwine-get-started`, `entwine-knowledge`, `entwine-deployment` | the `entwine` CLI |
| **MCP server** (`@entwine/mcp`) | A local stdio process exposing read-only tools, resources and prompts over the [context model](specs/context-schema.md) | Node 20+ |

Both sit on the same boundary: the Entwine CLI and `entwine context --json`. Neither
has its own parser, and the [architecture](architecture.md) is unchanged: Markdown stays the
source of truth.

## Skills only, or MCP too?

Skills alone are enough when the agent can run shell commands: they teach the
workflow (`entwine init --dry-run`, `entwine check`, `entwine context --json`,
`entwine setup --dry-run`). Add MCP when you want structured, bounded access
to documents, relationships and validation without shell access or without the agent
reading raw `context --json`. MCP never writes: applying `init` or `setup` is always
the agent running the CLI under its host's normal approval.

## Install the skills

Uses the open [`skills`](https://www.npmjs.com/package/skills) installer. Verified
with `skills` 1.7.2: `pnpm test:skills` installs from the local `skills/` directory into a
clean repository and checks the files, and the GitHub `tree/<branch>/skills` URL form was
installed from a pushed branch (the `main` URL works once this is merged):

```sh
# list what is available (3 skills)
npx skills add https://github.com/Andersseen/Entwine/tree/main/skills --list

# install all three for Codex and Claude Code, as copies, without prompts
npx skills add https://github.com/Andersseen/Entwine/tree/main/skills \
  --skill entwine-get-started entwine-knowledge entwine-deployment \
  --agent codex claude-code --copy -y

# or one skill
npx skills add https://github.com/Andersseen/Entwine/tree/main/skills --skill entwine-knowledge --agent codex --copy -y
```

| Host | Installed to | Notes |
| --- | --- | --- |
| Codex | `.agents/skills/<name>/` | shared with other agents that read `.agents/` |
| Claude Code | `.claude/skills/<name>/` | |

Behaviour to know: the installer adds `skills-lock.json`; it leaves other skills and
`AGENTS.md`/`CLAUDE.md` alone (tested), but **re-running replaces an installed skill with the
same name**, so do not hand-edit installed `entwine-*` skills. Do not point it at the
repository root: that also lists Entwine's own development skills. Always use the
`skills` subdirectory. Entwine's own tooling (Agentyx) is not needed by consumers.

## Register the MCP server

The server scopes itself to one repository. Root, in order: `--root <dir>`,
`ENTWINE_PROJECT_ROOT`, the working directory.

```sh
# Claude Code (documented syntax; options before the name, command after --)
claude mcp add entwine --scope project -- npx --yes @entwine/mcp --root .

# Codex
codex mcp add entwine -- npx --yes @entwine/mcp --root .
```

Equivalent config files: Claude Code `.mcp.json`
(`{"mcpServers":{"entwine":{"command":"npx","args":["--yes","@entwine/mcp","--root","."]}}}`)
and Codex `~/.codex/config.toml`
(`[mcp_servers.entwine]`, `command = "npx"`, `args = ["--yes","@entwine/mcp","--root","."]`).
Codex also reads a trusted project's `.codex/config.toml`. These host commands follow
each vendor's documentation; they were not executed in this repository's CI, which tests the
server with a real MCP client over stdio instead. Relative `--root .` depends on the host
launching from the repository; use an absolute path if unsure.

> **Availability.** `@entwine/mcp` is released together with `@entwine/cli` at the same version.
> Until a release containing it is on npm (`npm view @entwine/mcp versions`), `npx` fails with a 404.
> See [state](state.md#release-status). To use it from a source checkout, run
> `pnpm install && pnpm build:mcp` and register `node /path/to/Entwine/packages/mcp/dist/cli.js --root .`
> with `ENTWINE_BIN` pointing at a locally built `entwine`.

### What it exposes

Tools (read-only): `entwine_project_overview`, `entwine_list_documents` (filters and paging),
`entwine_read_document` (bounded slices), `entwine_related_documents`, `entwine_check`,
`entwine_init_preview`, `entwine_setup_preview` (both `--dry-run`).
Resources: `entwine://project/summary`, `entwine://document/{id}` with the id percent-encoded.
Prompts: `entwine_adopt`, `entwine_maintain_knowledge`, `entwine_prepare_publishing`.
Documents are addressed only by ids Entwine knows; no tool takes a path.

## Verify the installation

```sh
npx --yes @entwine/cli --version
npx --yes @entwine/mcp --version     # prints the version and supported context schema
claude mcp list                      # or `codex mcp list`; or /mcp inside the session
ls .agents/skills .claude/skills     # the three entwine-* directories
```

Then ask the agent: "Use Entwine to give me a project overview." It should call
`entwine_project_overview`, or without MCP run `entwine context --json`.

## Asking an agent to work

- *Inspect:* "List the Entwine decisions and show what links to the architecture document."
- *Validate:* "Run the Entwine check and explain each error without changing content."
- *Generate:* "Adopt Entwine here: preview `entwine init`, ask me, then fill the files only with facts you can verify in the repository." The agent writes Markdown; Entwine does not generate documentation from code.
- *Maintain:* use the `entwine_maintain_knowledge` prompt after a change.

## Publish with GitHub Pages, safely

1. `entwine check` passes.
2. `entwine setup --dry-run` (or `entwine_setup_preview`) shows `.github/workflows/entwine.yml`. Compare with existing workflows; setup never overwrites a file it did not create.
3. Confirm the pinned `@entwine/cli` version exists on npm; generated CI installs that version.
4. After approval run `entwine setup`, commit, push.
5. In the repository: Settings → Pages → Source = "GitHub Actions".
6. Verify the default-branch run succeeded and the Pages URL serves the docs before saying it is published.

See [deployment](deployment.md) for GitLab and Bitbucket.

## What it does not do

No `entwine publish`, hosting, or credential handling; no code-to-documentation
generation or drift detection; no AI; no HTTP service. The MCP server cannot write
files, run arbitrary commands, deploy, or change CI. Context `schema_version` is
experimental; the server supports `"0.4"` and refuses other versions rather than
guessing, so keep `@entwine/cli` and `@entwine/mcp` at the same version.
