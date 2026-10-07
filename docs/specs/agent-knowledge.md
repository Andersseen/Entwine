---
type: spec
---

# Agent knowledge

Entwine can discover agent-facing files in a repository and model them next to
documentation. This is optional, off by default, and never executes anything.
Entwine discovers, models, connects, and visualizes these files; it does not
interpret them the way an agent tool would.

## Configuration

`entwine.toml` at the project root (next to `docs/`) is optional. A missing file
and an empty file behave identically: compile `docs/`, discover nothing else.

```toml
[discovery]
agent_instructions = true
skills = true

[site]
include_agent_knowledge = false
```

| Key | Default | Meaning |
| --- | --- | --- |
| `discovery.agent_instructions` | `false` | Discover `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` |
| `discovery.skills` | `false` | Discover `SKILL.md` manifests and their folders |
| `site.include_agent_knowledge` | `false` | Publish discovered files in the generated site and graph |

Unknown sections or keys, and values of the wrong type, are errors
(`Invalid entwine.toml: ...`). The file must not be a symbolic link.

## Artifact kinds

Two orthogonal classifications exist. The **role** (`project`, `architecture`,
`state`, `roadmap`, `decision`, `spec`, `other`) says what a document means. The
**artifact kind** says what kind of file it is: `documentation`,
`agent_instructions`, or `skill`. Agent artifacts always have role `other`.

| Kind | Files | Structure recorded |
| --- | --- | --- |
| `agent_instructions` | `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` at any depth | path, convention, scope |
| `skill` | `SKILL.md` | path, folder, `name`, `description`, colocated resources |

Convention names are exact and case-sensitive. Provider conventions are data, not
code paths: the model has no provider-specific behavior.

## Scope

An instruction file's **scope** is the project-relative directory it sits in; the
empty scope is the project root. `packages/web/AGENTS.md` has scope
`packages/web`. Entwine records this structure and shows the nesting in the
Agents scopes view. It does not simulate which file a given tool would load or
how it would combine them.

## Skills

`name` and `description` come from `SKILL.md` frontmatter when present
(folded YAML descriptions are collapsed to one line); nothing is invented. A
missing `name` falls back to the first heading, then the folder name. Files in the
skill folder (up to depth four, 200 entries) are listed as resources. They are
never read, copied, or executed.

## Relationships

Links between documentation and agent artifacts become ordinary relationships,
in both directions: `docs/index.md` linking `../AGENTS.md`, or a skill linking
`../../../docs/architecture.md`. A link from an agent file to a missing file or
an unsupported scheme is a **warning**, not an error, and malformed frontmatter
in these files only warns. Links to existing non-knowledge files are repository
references.

## Discovery versus publication

1. **Discovery** (`[discovery]`): files enter the knowledge model.
2. **Projection**: `entwine context`, `entwine check`, and the structured model
   always include discovered files.
3. **Publication** (`site.include_agent_knowledge`): only when `true` do the
   generated site, its graph, and the `/__entwine/agents/` views include them.

With publication off, a documentation link to `AGENTS.md` renders as a labeled
repository path, exactly as it does without any configuration, and no output file
mentions the agent files. Publish only files you are happy to expose.

## Generated views

With publication on: `/__entwine/agents/` (overview), `.../instructions/`,
`.../skills/`, `.../scopes/`, one page per artifact, an Agents entry in the view
links, and agent nodes in the graph. Dot-directories are mapped to `dot-` route
segments (`.claude` becomes `dot-claude`).

## Limits

Discovery walks the project directory (not parent directories), skips `docs/`,
`node_modules`, `dist`, `target`, `vendor`, `.git`, and similar, never follows
symbolic links, reads files up to 1 MB, and stops at depth 12. It does not read
`.gitignore`. Images in agent files are not published.
