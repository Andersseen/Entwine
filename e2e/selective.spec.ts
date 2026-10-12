import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "@playwright/test";
import { canary, fixtureRoot, origins } from "./support/fixtures.ts";

function files(directory: string): string[] {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

test.describe("selective publication: one public skill, everything else private", () => {
  test.use({ baseURL: origins.selective });

  test("the complete dist tree carries only the selected skill", () => {
    const dist = join(fixtureRoot("selective"), "dist");
    const all = files(dist);
    expect(all.length).toBeGreaterThan(20);
    for (const file of all) {
      const text = readFileSync(file, "utf8");
      expect(text, file).not.toContain(canary);
      // Documentation that links a private file keeps its authored text, but no
      // generated route or artifact content for it exists.
      expect(text, file).not.toContain("dot-claude/skills/triage");
      expect(text, file).not.toContain("agents/instructions/");
      expect(file).not.toMatch(/instructions[\\/].+[\\/]index\.html$/);
      expect(file).not.toContain("triage-dataset-report");
    }
    expect(
      all.some((file) =>
        /skills[\\/].*export-catalogue[\\/]index\.html$/.test(file),
      ),
    ).toBe(true);
  });

  test("the public skill has a working page, source link and navigation", async ({
    page,
  }) => {
    await page.goto("/__entwine/agents/");
    await expect(page.locator("main")).toContainText("export-catalogue");
    await expect(page.locator("main")).not.toContainText("triage");
    await page
      .getByRole("link", { name: /export-catalogue/ })
      .first()
      .click();
    await expect(page.locator("main h1").first()).toBeVisible();
    await expect(page.locator("main")).toContainText(
      ".agents/skills/export-catalogue",
    );
    await expect(page.locator("main")).toContainText("fields.md");
  });

  test("private artifacts have no route, graph node or embedded data", async ({
    page,
  }) => {
    for (const path of [
      "/__entwine/agents/instructions/",
      "/__entwine/agents/skills/dot-claude/skills/triage-dataset-report/",
    ]) {
      const response = await page.goto(path);
      const body = (await response?.text()) ?? "";
      expect(body).not.toContain(canary);
      expect(body).not.toContain("Triage a dataset");
    }
    await page.goto("/__entwine/graph/");
    await expect(page.locator("#graph-app.is-live")).toBeVisible();
    await expect(
      page.locator(".graph-node.kind-agent_instructions"),
    ).toHaveCount(0);
    await expect(page.locator("a.graph-node.kind-skill")).toHaveCount(1);
    const payload = (await page.locator("#graph-data").textContent()) ?? "";
    expect(payload).toContain("export-catalogue");
    expect(payload).not.toMatch(/triage|AGENTS\.md|CLAUDE\.md|GEMINI\.md/);
  });
});
