# Context schema

`entwine context --json` writes the project's knowledge for downstream programs.
It comes from the same knowledge model as the site and graph; there is no separate
agent parser. See [architecture](../architecture.md#canonical-representation).

## Version

`schema_version` is currently `"0.4"` and is versioned separately from the Entwine
package. Version 0.4 adds `artifact` to every document (`documentation`,
`agent_instructions`, or `skill`) and, for discovered agent-facing artifacts, an
`agent` object (`convention`, `path`, `scope`, `name`, `description`,
`skill_directory`, `resources`). Documents from `docs/` have `artifact:
"documentation"` and no `agent`. Agent documents have ids prefixed `repo:`, role
`other`, and appear only when discovery is enabled by `entwine.toml`; they are
always in context regardless of site publication. Relationships may connect
documentation and agent artifacts. Consumers must check `schema_version`;
a 0.3 consumer that tolerates additional fields still reads documents correctly
but should ignore `repo:` ids it does not understand. Version 0.3 added `anchors` and `repository_references` to each document.
Repository references retain `path` (relative to the repository), `source`,
`line`, and original `destination`. They never become knowledge relationships.
A reference without provider metadata has an empty link `href`; renderers show
a labeled repository path instead. Consumers must check the schema version;
0.2 consumers that tolerate additional fields can ignore these two fields.
`content` now retains the exact original UTF-8 Markdown source, including
frontmatter and unknown metadata keys; consumers that need the body should parse
frontmatter rather than assuming it was removed.
README fallback may change a document route to `/` and its role to `project`. The schema is experimental before 1.0.

## Shape

```json
{
  "schema_version": "0.4",
  "documents": [
    {
      "id": "architecture.md",
      "title": "Architecture",
      "route": "/architecture/",
      "metadata": { "title": null, "type": "architecture", "status": null },
      "role": "architecture",
      "headings": [],
      "links": [],
      "anchors": [],
      "repository_references": [],
      "artifact": "documentation",
      "backlinks": [],
      "content": "..."
    }
  ],
  "relationships": [
    { "source": "index.md", "target": "architecture.md", "kind": "references" }
  ]
}
```

## Roles

`role` is one of `project`, `architecture`, `state`, `roadmap`, `decision`,
`spec`, or `other`. It is derived (see the [convention](../convention.md#roles-and-how-they-are-found)
and [the decision](../decisions/derived-roles.md)). `metadata.type` is always the
author's original text. Consumers that need to know what is true now read
`state`; intended direction is `roadmap`.

An agent artifact differs only in these fields:

```json
{
  "id": "repo:packages/web/AGENTS.md",
  "title": "packages/web/AGENTS.md",
  "role": "other",
  "artifact": "agent_instructions",
  "agent": {
    "convention": "agents_md",
    "path": "packages/web/AGENTS.md",
    "scope": "packages/web",
    "name": null,
    "description": null,
    "skill_directory": null,
    "resources": []
  }
}
```

`path` and `skill_directory` are repository-relative; `scope` is relative to the
project (`""` is the project root) and is `null` for skills. See
[agent knowledge](agent-knowledge.md).

## Guarantees

Output is deterministic: documents are ordered by path and relationships are
unique and sorted. The Markdown form (`entwine context` without `--json`) shows
the same roles.
