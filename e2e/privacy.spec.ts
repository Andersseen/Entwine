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

test.describe("privacy boundary: discovery on, publication off", () => {
  test.use({ baseURL: origins.private });

  test("the filesystem output contains no agent knowledge", () => {
    const dist = join(fixtureRoot("private"), "dist");
    const all = files(dist);
    expect(all.length).toBeGreaterThan(5);
    expect(all.filter((file) => /[\\/]agents[\\/]/.test(file))).toEqual([]);
    for (const file of all) {
      const text = readFileSync(file, "utf8");
      expect(text, file).not.toContain(canary);
      expect(text, file).not.toMatch(/data-node="repo:|"id":"repo:/);
      if (file.endsWith(".html"))
        expect(text, file).not.toMatch(
          /kind-(skill|agent_instructions)|"kind":"(skill|agent_instructions)"/,
        );
    }
  });

  test("agent routes are not served", async ({ page }) => {
    for (const path of [
      "/__entwine/agents/",
      "/__entwine/agents/skills/",
      "/__entwine/agents/instructions/",
      "/__entwine/agents/scopes/",
    ]) {
      const response = await page.goto(path);
      expect(response?.status(), path).toBe(404);
    }
  });

  test("the graph and Knowledge view carry no agent content", async ({
    page,
  }) => {
    await page.goto("/__entwine/graph/");
    await expect(page.locator("#graph-app.is-live")).toBeVisible();
    await expect(
      page.locator(
        ".graph-node.kind-skill, .graph-node.kind-agent_instructions",
      ),
    ).toHaveCount(0);
    await expect(
      page.locator(
        '[data-filter-kind="skill"], [data-filter-kind="agent_instructions"]',
      ),
    ).toHaveCount(0);
    const payload = (await page.locator("#graph-data").textContent()) ?? "";
    expect(payload).not.toContain(canary);
    expect(payload).not.toMatch(/AGENTS\.md|SKILL\.md|CLAUDE\.md/);
    await page.goto("/__entwine/knowledge/");
    await expect(page.locator("main")).not.toContainText(
      /Agent knowledge|skills/i,
    );
    await expect(page.getByRole("link", { name: "Agents" })).toHaveCount(0);
  });

  test("a deep link to an agent artifact selects nothing", async ({ page }) => {
    await page.goto(
      `/__entwine/graph/?focus=${encodeURIComponent("repo:AGENTS.md")}`,
    );
    await expect(page.locator("#graph-app.is-live")).toBeVisible();
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(0);
  });
});
