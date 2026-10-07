---
type: project
---

# Harbor

Harbor is a small, fictional service for teams to catalogue shared datasets. This
site is the project knowledge for that imaginary project, and it is a worked
example of Entwine and its Knowledge Convention.

## How to read this demo

This is what Entwine compiles from plain Markdown under `docs/`. There is no
documentation framework or theme, and `docs/` needs no configuration. The optional
`entwine.toml` only switches on agent knowledge discovery (see below).

1. **Browse the knowledge.** Each page carries a role badge: Architecture,
   Current state, Roadmap, Decision, or Specification.
2. **Open Knowledge** in the sidebar. It lists which recommended areas exist and
   links to each document.
3. **Open Graph** in the sidebar. It shows how those documents reference each
   other. Every link you see in the text is a relationship, and the
   **Referenced by** list at the bottom of each page is derived from them.
4. Run `entwine context --json` to see the same knowledge as structured data
   for tools and coding agents, each document carrying its role.

## Project knowledge

| Area | Document | Question it answers |
| --- | --- | --- |
| Architecture | [Harbor architecture](architecture.md) and the [data model](design/data-model.md) | How is Harbor built today? |
| Current state | [Current state](state.md) | What is true right now? |
| Roadmap | [Roadmap](roadmap.md) | What happens next? |
| Decisions | [Static output](decisions/static-output.md), [Read-only first](decisions/read-only-first.md), [Anonymous public reads](decisions/anonymous-reads.md) | Why is it this way? |
| Specifications | [Authentication](specs/authentication.md), [Search](specs/search.md), [Catalogue export](specs/catalogue-export.md) | What do capabilities do? |
| Other knowledge | [Operations runbook](operations/runbook.md) | Anything else worth keeping. |

The runbook is a free-form guide with its own `type`, and the data model lives in
`design/` rather than `architecture.md`; Entwine recognizes it as architecture
from its metadata. The convention is recommended, not required.

## Agent knowledge

Besides durable documentation, Harbor keeps agent-facing files in the repository:
the [project instructions](../AGENTS.md), scoped instructions for the
[catalogue API](../services/catalogue-api/AGENTS.md), and two skills such as
[triage a dataset report](../.claude/skills/triage-dataset-report/SKILL.md).
The [`entwine.toml`](../entwine.toml) in this example turns discovery on and
publishes them, so the **Agents** view and the graph show how they connect to
the documents above. Without that file Entwine would ignore them.

## Local workflow

```sh
entwine check
entwine dev
```

Your Markdown remains the source of truth. See [CommonMark](https://commonmark.org)
for the underlying Markdown syntax.

## Repository and sections

The [repository guide](../README.md) explains the Harbor example. Browse the
[design documents](design/) and [operations documents](operations/) as generated directory indexes.

![Harbor knowledge mark](assets/harbor.svg)
