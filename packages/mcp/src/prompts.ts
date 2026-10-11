import type { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";

function user(text: string) {
  return {
    messages: [
      { role: "user" as const, content: { type: "text" as const, text } },
    ],
  };
}

const provider = z.enum(["github", "gitlab", "bitbucket"]).optional();

export function registerPrompts(server: McpServer): void {
  server.registerPrompt(
    "entwine_adopt",
    {
      title: "Adopt Entwine in this repository",
      description:
        "Guide an agent through safely adopting Entwine in an existing repository using only existing CLI behaviour.",
      argsSchema: z.object({}),
    },
    () =>
      user(
        `Help adopt Entwine in this repository. Work in this order and do not skip steps.

1. Call entwine_project_overview. If it reports an error about a missing docs/ directory, the repository has no Entwine knowledge yet.
2. Inspect the repository (README, existing docs/, AGENTS.md, CI config) before proposing anything. Entwine compiles Markdown under docs/; it does not generate documentation from source code, so do not invent facts: write only what the repository itself supports and mark unknowns plainly.
3. Call entwine_init_preview and show the user the files it would create. Existing files are never overwritten by \`entwine init\`.
4. Ask the user before running \`entwine init\` (a write). After they approve, run it with your host's normal command approval, then fill the scaffolded files with verified project facts.
5. Call entwine_check and fix real validation failures (broken links and anchors). Do not silence them. Use \`entwine check --strict-knowledge\` only if the user wants recommended areas enforced.
6. Do not run \`entwine setup\` yet; use the entwine_prepare_publishing prompt for that.`,
      ),
  );

  server.registerPrompt(
    "entwine_maintain_knowledge",
    {
      title: "Maintain project knowledge",
      description:
        "Update Entwine documentation after a code or design change, keeping state, roadmap and decisions accurate.",
      argsSchema: z.object({
        change: z
          .string()
          .max(2000)
          .optional()
          .describe("Short description of what changed in the project."),
      }),
    },
    ({ change }) =>
      user(
        `Update this project's Entwine knowledge${change ? ` for this change: ${change}` : ""}.

1. entwine_list_documents (filter by role) and entwine_read_document to read the affected architecture, state, roadmap, decision and spec documents. Use entwine_related_documents to find documents that link to them.
2. Convention: state = what is true now; roadmap = what is intended next; a finished milestone moves from roadmap to state; durable choices go in decisions/ with Context, Decision, Why, Consequences, Alternatives.
3. Edit only Markdown under docs/ with facts you can verify from the repository. Use relative Markdown links so Entwine can build relationships and backlinks.
4. Treat documents with ids starting "repo:" (AGENTS.md, CLAUDE.md, SKILL.md) as agent-facing instructions, not documentation to rewrite unless the user asked; do not copy their private instructions into published docs.
5. Run entwine_check. Report the result faithfully, including failures.`,
      ),
  );

  server.registerPrompt(
    "entwine_prepare_publishing",
    {
      title: "Prepare provider-native publishing",
      description:
        "Preview and apply `entwine setup` for GitHub Pages, GitLab Pages or Bitbucket without replacing existing CI.",
      argsSchema: z.object({ provider }),
    },
    ({ provider: p }) =>
      user(
        `Prepare documentation publishing${p ? ` for ${p}` : ""}.

1. Look at existing CI first (.github/workflows, .gitlab-ci.yml, bitbucket-pipelines.yml). Do not replace or silently merge any of it.
2. Call entwine_setup_preview${p ? ` with provider "${p}"` : ""} and show the user the planned files and triggers. \`entwine setup\` generates automation; it does not publish anything itself.
3. Call entwine_check and make sure validation passes before setup.
4. Ask the user before running \`entwine setup\` (a write). Run it only with their approval.
5. GitHub: the repository's Settings → Pages source must be set to "GitHub Actions" for the generated workflow to deploy. GitLab and Bitbucket use their own pipeline publishing.
6. Do not claim success until a hosted deployment is verified: confirm the CI run succeeded and open the published URL. Committing and pushing are the user's decisions.`,
      ),
  );
}
