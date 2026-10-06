---
type: spec
---

# Entwine Knowledge Convention v0.1

A small, open convention for durable project knowledge. It is recommended, not
required: any Markdown under `docs/` compiles whether or not it follows it.

Entwine is opinionated by default and flexible by design. The convention is the
golden path; it is not schema validation. It is a convention that may earn
adoption over time, not an industry standard.

## Why a convention

Many ecosystems already have predictable places for specific concerns:
`README.md` introduces a project to humans, `AGENTS.md` instructs coding agents,
`SKILL.md` holds reusable procedures, MCP exposes tools, and OpenSpec-style
tooling manages proposed changes. There is much less consistency for durable
project knowledge: architecture, current state, roadmap, decisions, specifications,
and the relationships between them. Repositories invent `ARCHITECTURE_V2.md`,
`CURRENT_STATE.md`, `context.md`, and `docs/notes/`. Humans and agents then
reconstruct the same facts repeatedly.

## Recommended structure

```text
docs/
├── index.md
├── architecture.md
├── state.md
├── roadmap.md
├── decisions/
│   └── *.md
└── specs/
    └── *.md
```

| Area | Answers | Role |
| --- | --- | --- |
| `index.md` | What is this project, and where is its knowledge? | `project` |
| `architecture.md` | How is the system structured **today**? | `architecture` |
| `state.md` | What is true **right now**? | `state` |
| `roadmap.md` | What is intended **next**? | `roadmap` |
| `decisions/` | What was decided, and why? | `decision` |
| `specs/` | What do capabilities do, durably? | `spec` |

### index.md

The project knowledge entry point: what the project is, how its knowledge is
organized, and where the important areas are. It is not a copy of the README.
The README is the public introduction; `docs/index.md` is the entry point to
durable knowledge.

### architecture.md

How the system is structured today: major boundaries, modules, services or
packages, runtime topology, data flow, dependency direction, and important
technical constraints. It describes the current architecture, not future wishes.

### state.md

What is true about the project now: implemented and partially implemented
capabilities, current integrations, known limitations, storage, runtime, and
deployment choices, and active constraints. It is especially valuable to coding
agents, who otherwise reconstruct current reality from commits and code.

### roadmap.md

Intended future direction. Keep the distinction sharp: **state is current
reality, roadmap is future intent.** Moving a capability from one to the other is
exactly what a finished milestone looks like.

### decisions/

Durable decisions and why they were made. A lightweight shape is enough:
Context, Decision, Why, Consequences, Alternatives. There is no ADR tooling to
adopt.

### specs/

Durable descriptions of existing or intended capabilities. See
[coexistence](#coexistence-with-existing-conventions) for how this differs from a
change lifecycle.

## Roles and how they are found

Entwine derives a **knowledge role** for every document. The role is separate from
the author's `type`, which stays open-ended and is always preserved (and exposed
as-is in [structured context](specs/context-schema.md)).

Precedence:

1. A recognized frontmatter `type` (`project`, `architecture`, `state`,
   `roadmap`, `decision`, `spec`, plus the aliases `decisions`, `adr`, `specs`,
   `specification`; case-insensitive).
2. The canonical path: `index.md`, `architecture.md`, `state.md`, `roadmap.md`,
   `decisions/**/*.md`, `specs/**/*.md`, relative to `docs/`.
3. Otherwise `other`.

A landing page directly inside `decisions/` or `specs/` (its `index.md`) explains
the section; it does not count as a decision or specification unless its `type`
says so. Unknown types such as `guide` or `plan` keep their text and get the role
`other`.

So an existing repository does not need renames:

```yaml
---
type: architecture
---
```

on `docs/design/system.md` is understood as architecture.

If a recognized `type` disagrees with a canonical path, for example
`architecture.md` declaring `type: roadmap`, Entwine uses the explicit type and
emits a warning. It never fails the build.

## Coverage and strictness

`entwine check` prints which recommended areas are represented. This is
**presence only**. It is not a quality score, and Entwine never prints a
percentage. Missing areas are warnings:

```text
✓ Architecture
  docs/architecture.md

⚠ Specifications
  No specification documents found (recommended: docs/specs/)
```

Teams that want enforcement can opt in with `entwine check --strict-knowledge`,
which fails when any recommended area is missing. Plain `entwine check` stays
permissive, and `entwine build` is never affected. There is no configuration file.

## Adopting it

`entwine init` creates only the missing recommended files, using short templates
that ask questions and never state facts. It does not read your code, does not use
AI, never overwrites, and is safe to repeat. Areas already represented through
custom filenames and types are left alone.

## Coexistence with existing conventions

Entwine complements these; it does not ingest or replace them.

| Convention | Responsibility |
| --- | --- |
| `README.md` | Project introduction |
| `AGENTS.md` / `CLAUDE.md` | Agent operating instructions |
| `SKILL.md` | Reusable procedures |
| MCP | Runtime tools and capabilities |
| OpenSpec / SDD | Change lifecycle |
| Entwine | Durable project knowledge |

OpenSpec and similar tooling manage the lifecycle of a proposed change. An Entwine
spec is durable knowledge about a capability. A completed change may eventually
leave behind durable Entwine knowledge, but Entwine implements no change
workflow. Agent instruction files can link to Entwine documents, and Entwine
documents can link back; they are ordinary Markdown links.

## Example

This repository follows its own convention: [index](index.md),
[architecture](architecture.md), [state](state.md), [roadmap](roadmap.md),
[decisions](decisions/derived-roles.md), and [specs](specs/context-schema.md).
The published [demo](deployment.md#official-showcase) shows the same shape for a
fictional project.
