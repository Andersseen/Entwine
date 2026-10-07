import { expect, test } from "@playwright/test";

test.describe("progressive enhancement (JavaScript disabled)", () => {
  test.use({ javaScriptEnabled: false });

  test("the graph page still shows a static graph, a node list and relationships", async ({
    page,
  }) => {
    await page.goto("/__entwine/graph/");
    await expect(page.locator("svg.graph")).toBeVisible();
    expect(await page.locator("a.graph-node").count()).toBeGreaterThan(10);
    await expect(page.locator(".graph-controls")).toBeHidden();
    // The complete text alternative follows the graph.
    await expect(
      page.locator("summary", { hasText: /Relationships \(\d+\)/ }),
    ).toBeVisible();
    const link = page.locator(
      'a.graph-node[data-node="specs/authentication.md"]',
    );
    await link.click();
    await expect(page).toHaveURL(/\/specs\/authentication\/$/);
  });

  test("documentation, Knowledge and Agents remain navigable", async ({
    page,
  }) => {
    await page.goto("/specs/authentication/");
    await page
      .getByRole("region", { name: "Referenced by" })
      .getByRole("link")
      .first()
      .click();
    await page.goto("/__entwine/knowledge/");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Project knowledge",
    );
    await page.goto("/__entwine/agents/skills/");
    await expect(page.locator("main")).toContainText("export-catalogue");
  });
});
