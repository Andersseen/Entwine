# Harbor agent instructions

Harbor is a read-only dataset catalogue. Read the [architecture](docs/architecture.md)
and the [current state](docs/state.md) before changing behavior; the roadmap is
intent, not fact.

## Ground rules

- Harbor is read-only first: see the [decision](docs/decisions/read-only-first.md).
  Do not add write endpoints without a new decision record.
- Authentication follows the [authentication spec](docs/specs/authentication.md).
- Keep `docs/state.md` true. If you change what works today, update it.

## Where things live

| Area | Instructions |
| --- | --- |
| Catalogue API | [services/catalogue-api/AGENTS.md](services/catalogue-api/AGENTS.md) |
| Web client | [services/web/GEMINI.md](services/web/GEMINI.md) |

## Skills

- [Triage a dataset report](.claude/skills/triage-dataset-report/SKILL.md)
- [Export the catalogue](.agents/skills/export-catalogue/SKILL.md)
