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
| `site.include_agent_knowledge` | `false` | Publish **every** discovered file in the generated site and graph |
| `site.public_agent_paths` | `[]` | Publish **only** these discovered files (source-only until the next release; see below) |

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
3. **Publication** (`site.include_agent_knowledge` or `site.public_agent_paths`): only
   what is explicitly published reaches the generated site, its graph, and the
   `/__entwine/agents/` views.

With publication off, a documentation link to `AGENTS.md` renders as a labeled
repository path, exactly as it does without any configuration, and no output file
mentions the agent files. Publish only files you are happy to expose.

### Selective publication

`site.public_agent_paths` publishes a named subset, for example public skills while
internal `AGENTS.md` files stay private:

```toml
[discovery]
agent_instructions = true
skills = true

[site]
public_agent_paths = [
  ".agents/skills/entwine-get-started/SKILL.md",
  ".agents/skills/entwine-knowledge/SKILL.md",
]
```

Rules, all enforced as errors rather than silently widened:

- Entries are exact, `/`-separated, repository-relative paths of discovered artifacts
  (the `agent.path` in `entwine context --json`). No globs, directories, absolute
  paths, `.`/`..` segments, backslashes or duplicates.
- The file name must be `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` or `SKILL.md`, its
  discovery switch must be on, and the file must actually have been discovered (so a
  missing file, a symlink escape, `docs/` and skipped directories are all rejected).
- It contradicts `include_agent_knowledge = true`; set one or the other.
- An empty list (the default) publishes nothing. Discovery alone never publishes.

The filter is applied at the projection boundary. The full model, `entwine context`
and `check` are unchanged, and the MCP server still sees every discovered artifact
locally. Pages, navigation, the graph (nodes, edges, files), backlinks, references,
source links, rendered HTML and embedded graph data contain only the selected
artifacts. A selected skill that links a private file keeps its link text, but the
anchor is removed so no private route is revealed; documentation that links a private
artifact renders it as an ordinary repository path, as with publication off. With no
selected artifact there are no `/__entwine/agents/` content pages.

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
