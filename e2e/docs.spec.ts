import { expect, test } from "@playwright/test";
import { trackErrors } from "./support/graph.ts";

test.describe("documentation navigation", () => {
  test("home, navigation and nested docs work without console errors", async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await page.goto("/");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("Harbor");
    await page
      .getByRole("navigation", { name: "Documentation" })
      .getByRole("link", { name: "Authentication" })
      .click();
    await expect(page).toHaveURL(/\/specs\/authentication\/$/);
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Authentication",
    );
    await expect(
      page
        .getByRole("navigation", { name: "Documentation" })
        .getByRole("link", { name: "Authentication" }),
    ).toHaveAttribute("aria-current", "page");
    expect(errors()).toEqual([]);
  });

  test("heading fragments scroll to the heading and the table of contents links to them", async ({
    page,
  }) => {
    await page.goto("/specs/authentication/#failure-behavior");
    await expect(page.locator("#failure-behavior")).toBeInViewport();
    const toc = page.getByRole("navigation", { name: "On this page" });
    await expect(
      toc.getByRole("link", { name: "Session boundary" }),
    ).toHaveAttribute("href", "#session-boundary");
    await toc.getByRole("link", { name: "Session boundary" }).click();
    await expect(page).toHaveURL(/#session-boundary$/);
  });

  test("links between pages and cross-page fragments resolve", async ({
    page,
  }) => {
    await page.goto("/specs/authentication/");
    await page.locator("article").getByRole("link", { name: "search" }).click();
    await expect(page).toHaveURL(/\/specs\/search\/#query-contract$/);
    await expect(page.locator("#query-contract")).toBeInViewport();
  });

  test("references and backlinks are listed and navigable", async ({
    page,
  }) => {
    await page.goto("/specs/authentication/");
    const references = page.getByRole("region", { name: "References" });
    await expect(
      references.getByRole("link", { name: "Roadmap" }),
    ).toBeVisible();
    const backlinks = page.getByRole("region", { name: "Referenced by" });
    await expect(
      backlinks.getByRole("link", { name: "Harbor architecture" }),
    ).toBeVisible();
    await backlinks.getByRole("link", { name: "Harbor architecture" }).click();
    await expect(page).toHaveURL(/\/architecture\/$/);
  });

  test("source links point at the repository file", async ({ page }) => {
    await page.goto("/specs/authentication/");
    const source = page.locator("p.source a");
    await expect(source).toHaveAttribute(
      "href",
      /^https:\/\/github\.com\/example\/harbor\/blob\/[0-9a-f]{40}\/docs\/specs\/authentication\.md$/,
    );
  });

  test("Knowledge view lists documents by role and links onward", async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await page.goto("/__entwine/knowledge/");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Project knowledge",
    );
    await page.getByRole("link", { name: "Authentication" }).first().click();
    await expect(page).toHaveURL(/\/specs\/authentication\/$/);
    await page.goto("/__entwine/knowledge/");
    await page
      .getByRole("link", { name: /agent instructions, skills, and scopes/i })
      .click();
    await expect(page).toHaveURL(/\/__entwine\/agents\/$/);
    expect(errors()).toEqual([]);
  });

  test("unknown routes are not served", async ({ page }) => {
    const response = await page.goto("/no-such-page/");
    expect(response?.status()).toBe(404);
  });
});
