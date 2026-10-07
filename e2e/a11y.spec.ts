import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import { openGraph } from "./support/graph.ts";

const pages = [
  "/",
  "/specs/authentication/",
  "/__entwine/knowledge/",
  "/__entwine/agents/",
  "/__entwine/agents/skills/",
  "/__entwine/graph/",
];

for (const scheme of ["light", "dark"] as const) {
  test.describe(`axe, ${scheme} scheme`, () => {
    test.use({ colorScheme: scheme });
    for (const path of pages) {
      test(`${path} has no detectable violations`, async ({ page }) => {
        if (path.endsWith("graph/")) await openGraph(page);
        else await page.goto(path);
        const results = await new AxeBuilder({ page }).analyze();
        expect(
          results.violations.map(
            (v) =>
              `${v.id}: ${v.nodes
                .map((n) => n.target.join(" "))
                .slice(0, 3)
                .join(", ")}`,
          ),
        ).toEqual([]);
      });
    }
  });
}

test.describe("keyboard and structure", () => {
  test("the skip link is the first tab stop, becomes visible and moves focus to main", async ({
    page,
  }) => {
    await page.goto("/specs/authentication/");
    await page.keyboard.press("Tab");
    const skip = page.getByRole("link", { name: "Skip to content" });
    await expect(skip).toBeFocused();
    await expect(skip).toBeInViewport();
    await page.keyboard.press("Enter");
    await expect(page.locator("main")).toBeFocused();
  });

  test("pages have one main landmark, one h1 and a labelled navigation", async ({
    page,
  }) => {
    for (const path of pages) {
      await page.goto(path);
      await expect(page.getByRole("main"), path).toHaveCount(1);
      await expect(page.getByRole("heading", { level: 1 }), path).toHaveCount(
        path === "/__entwine/agents/skills/" ? 1 : 1,
      );
      for (const nav of await page.getByRole("navigation").all()) {
        const label = await nav.getAttribute("aria-label");
        expect(label, `${path}: unlabeled <nav>`).toBeTruthy();
      }
    }
  });

  test("focus is visible on links, graph controls and the graph stage", async ({
    page,
  }) => {
    await openGraph(page);
    for (const target of [
      page.getByRole("button", { name: "Fit", exact: true }),
      page.locator(".graph-scroll"),
      page.locator("a.graph-node").first(),
    ]) {
      await target.focus();
      const outline = await target.evaluate((el) => {
        const style = getComputedStyle(el);
        return `${style.outlineStyle}|${style.outlineWidth}|${style.boxShadow}`;
      });
      const shape = await target.evaluate((el) => {
        const s = el.querySelector(".shape");
        return s ? getComputedStyle(s).strokeWidth : "";
      });
      expect(
        !/^none\|/.test(outline) ||
          outline.split("|")[2] !== "none" ||
          Number.parseFloat(shape) >= 3,
        `no visible focus indicator: ${outline} ${shape}`,
      ).toBe(true);
    }
  });

  test("graph nodes are keyboard-selectable and the inspector announces politely", async ({
    page,
  }) => {
    await openGraph(page);
    await expect(page.locator(".graph-inspector")).toHaveAttribute(
      "aria-live",
      "polite",
    );
    const first = page.locator("a.graph-node:not(.is-off)").first();
    await first.focus();
    await page.keyboard.press("Enter");
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(1);
    await expect(page.locator(".graph-inspector h2")).toBeVisible();
  });

  test("the graph stage is labelled and the SVG has title and description", async ({
    page,
  }) => {
    await openGraph(page);
    await expect(page.locator(".graph-scroll")).toHaveAttribute(
      "aria-label",
      /Document graph/,
    );
    await expect(page.locator("svg.graph > title")).toHaveText(
      "Project knowledge graph",
    );
    await expect(page.locator("svg.graph > desc")).toContainText(
      "directed references",
    );
  });

  test.describe("reduced motion", () => {
    test.use({ reducedMotion: "reduce" });
    test("dragging works without animation and Reset snaps back", async ({
      page,
    }) => {
      await openGraph(page);
      const arch = page.locator('a.graph-node[data-node="architecture.md"]');
      await arch.scrollIntoViewIfNeeded();
      const box = (await arch.locator(".shape").boundingBox())!;
      const before = await arch.getAttribute("transform");
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + 100, box.y + 80, { steps: 5 });
      await page.mouse.up();
      await expect.poll(() => arch.getAttribute("transform")).not.toBe(before);
      await page.getByRole("button", { name: "Reset layout" }).click();
      await expect.poll(() => arch.getAttribute("transform")).toBe(before);
    });
  });
});
