import { expect, test } from "@playwright/test";
import { inspector, openGraph, trackErrors } from "./support/graph.ts";

test.describe("Agent Knowledge (published)", () => {
  test("overview, instructions, skills and scopes are browsable", async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await page.goto("/__entwine/agents/");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Agent knowledge",
    );
    await expect(page.locator("main")).toContainText(
      "4 instruction files in 3 scopes, and 2 skills",
    );

    const views = page.getByRole("navigation", { name: /agent/i });
    await page.goto("/__entwine/agents/instructions/");
    await expect(page.locator("main")).toContainText(
      "services/catalogue-api/AGENTS.md",
    );
    await page.goto("/__entwine/agents/skills/");
    await expect(page.locator("main")).toContainText("export-catalogue");
    await expect(page.locator("main")).toContainText("triage-dataset-report");
    await page.goto("/__entwine/agents/scopes/");
    await expect(page.locator("main")).toContainText("services/catalogue-api");
    void views;
    expect(errors()).toEqual([]);
  });

  test("an artifact page shows its source, and resources are listed not published", async ({
    page,
  }) => {
    await page.goto("/__entwine/agents/skills/");
    await page.getByRole("link", { name: "export-catalogue" }).first().click();
    await expect(page.getByRole("heading", { level: 1 }).first()).toContainText(
      "export-catalogue",
    );
    await expect(page.locator("main")).toContainText(
      ".agents/skills/export-catalogue",
    );
    await expect(page.locator("main")).toContainText("fields.md");
    const response = await page.request.get(
      "/__entwine/agents/skills/dot-agents/skills/export-catalogue/references/fields.md",
    );
    expect(response.status()).toBe(404);
  });

  test("navigates between agents, documentation and the graph", async ({
    page,
  }) => {
    await page.goto("/__entwine/agents/");
    await page
      .getByRole("navigation", { name: "Entwine views" })
      .getByRole("link", { name: "Graph" })
      .click();
    await expect(page).toHaveURL(/\/__entwine\/graph\/$/);
    await expect(
      page.locator("a.graph-node.kind-skill").first(),
    ).toBeAttached();
    await page.goto("/__entwine/graph/");
    await page
      .getByRole("navigation", { name: "Entwine views" })
      .getByRole("link", { name: "Documentation" })
      .click();
    await expect(page).toHaveURL(/\/$/);
  });

  test("a skill node in the graph opens its artifact page", async ({
    page,
  }) => {
    await openGraph(
      page,
      `?focus=${encodeURIComponent("repo:.agents/skills/export-catalogue/SKILL.md")}`,
    );
    await expect(inspector(page).getByRole("heading", { level: 2 })).toHaveText(
      "export-catalogue",
    );
    await inspector(page).getByRole("link", { name: "Open page" }).click();
    await expect(page).toHaveURL(/\/__entwine\/agents\/skills\//);
  });
});
