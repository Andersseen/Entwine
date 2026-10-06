//! Scaffold templates. They ask questions; they never state project facts.
use entwine_core::{humanize, KnowledgeRole};

pub(crate) const ARCHITECTURE: &str = "---
type: architecture
---

# Architecture

## Overview

Describe the major parts of the system and how they interact.

## Boundaries

Describe important module, service, or package boundaries.

## Data flow

Describe how data moves through the system.

## Constraints

Record important architectural constraints.
";

pub(crate) const STATE: &str = "---
type: state
---

# Current State

## Implemented

What works today?

## In progress

What is partially implemented?

## Known limitations

What important limitations currently exist?
";

pub(crate) const ROADMAP: &str = "---
type: roadmap
---

# Roadmap

## Next

What is the next justified milestone?

## Later

What is intentionally deferred?
";

pub(crate) const DECISIONS_INDEX: &str = "---
title: Decisions
---

# Decisions

Durable decisions and why they were made. Add one Markdown file per decision in
this directory, for example `0001-short-title.md`. Files here are recognized as
decisions automatically. A lightweight shape is enough:

```md
# Short decision title

## Context

What situation or constraint made a decision necessary?

## Decision

What was decided?

## Why

Why is this the right choice here?

## Consequences

What becomes easier or harder?

## Alternatives

What else was considered, and why was it not chosen?
```
";

pub(crate) const SPECS_INDEX: &str = "---
title: Specifications
---

# Specifications

Durable descriptions of project capabilities, existing or intended. Add one
Markdown file per capability in this directory, for example `search.md`. Files
here are recognized as specifications automatically.

A specification describes what a capability is and how it behaves. It is
durable project knowledge, not a change proposal; change lifecycles belong to
tools such as OpenSpec.

```md
# Capability name

## Purpose

What does this capability do, and for whom?

## Behavior

How does it behave, including edge cases and failures?

## Related

Link to the architecture, decisions, or other specifications it depends on.
```
";

/// The entry point links only to files that exist or are being created beside it.
pub(crate) fn index(project_name: &str, links: &[(KnowledgeRole, String, bool)]) -> String {
    let mut text = format!(
        "---\ntype: project\n---\n\n# {}\n\nWhat is this project, and who is it for? Replace this sentence with a short answer.\n\n## Project knowledge\n\nThis page is the entry point to the project's durable knowledge. The repository README remains the public introduction.\n\n",
        humanize(project_name)
    );
    for (role, path, _created) in links {
        let (label, hint) = match role {
            KnowledgeRole::Architecture => ("Architecture", "how the system is structured today"),
            KnowledgeRole::State => ("Current state", "what is true right now"),
            KnowledgeRole::Roadmap => ("Roadmap", "intended future direction"),
            KnowledgeRole::Decision => ("Decisions", "why important choices were made"),
            KnowledgeRole::Spec => ("Specifications", "durable descriptions of capabilities"),
            _ => continue,
        };
        text.push_str(&format!("- [{label}]({path}): {hint}\n"));
    }
    text
}
