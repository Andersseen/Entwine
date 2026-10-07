import { expect, test } from "@playwright/test";
import {
  center,
  distance,
  edgeEnds,
  inspector,
  node,
  openGraph,
  position,
  trackErrors,
  view,
  visibleNodes,
} from "./support/graph.ts";

const AUTH = "specs/authentication.md";
const ARCH = "architecture.md";

test.describe("graph core interaction", () => {
  test("loads as an enhanced SVG with usable controls", async ({ page }) => {
    const errors = trackErrors(page);
    await openGraph(page);
    await expect(page.locator("svg.graph")).toBeVisible();
    expect(await page.locator("a.graph-node").count()).toBeGreaterThan(10);
    for (const name of ["Zoom in", "Zoom out", "Fit", "Reset layout"]) {
      await expect(
        page.getByRole("button", { name, exact: true }),
      ).toBeEnabled();
    }
    // Focus needs a selection first.
    await expect(
      page.getByRole("button", { name: "Focus neighbors" }),
    ).toBeDisabled();
    expect(errors()).toEqual([]);
  });

  test("zoom buttons and wheel change the viewport scale", async ({ page }) => {
    await openGraph(page);
    const initial = (await view(page)).k;
    await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    await expect
      .poll(async () => (await view(page)).k)
      .toBeGreaterThan(initial);
    await page.getByRole("button", { name: "Zoom out", exact: true }).click();
    await page.getByRole("button", { name: "Zoom out", exact: true }).click();
    await expect.poll(async () => (await view(page)).k).toBeLessThan(initial);

    const stage = page.locator(".graph-scroll");
    await stage.scrollIntoViewIfNeeded();
    const box = (await stage.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    const before = (await view(page)).k;
    await page.mouse.wheel(0, -300);
    await expect.poll(async () => (await view(page)).k).toBeGreaterThan(before);
  });

  test("dragging the background pans the viewport", async ({ page }) => {
    await openGraph(page);
    const before = await view(page);
    await page.locator(".graph-scroll").scrollIntoViewIfNeeded();
    const stage = (await page.locator(".graph-scroll").boundingBox())!;
    const startX = stage.x + 6;
    const startY = stage.y + 6;
    await page.mouse.move(startX, startY);
    await page.mouse.down();
    await page.mouse.move(startX + 90, startY + 60, { steps: 6 });
    await page.mouse.up();
    await expect
      .poll(async () => (await view(page)).x)
      .toBeCloseTo(before.x + 90, 0);
    expect((await view(page)).y).toBeCloseTo(before.y + 60, 0);
    expect((await view(page)).k).toBe(before.k);
  });

  test("keyboard pans and zooms the focused stage", async ({ page }) => {
    await openGraph(page);
    await page.locator(".graph-scroll").focus();
    const before = await view(page);
    await page.keyboard.press("ArrowLeft");
    await expect.poll(async () => (await view(page)).x).toBe(before.x + 40);
    await page.keyboard.press("+");
    await expect
      .poll(async () => (await view(page)).k)
      .toBeGreaterThan(before.k);
    await page.keyboard.press("0");
    await expect
      .poll(async () => (await view(page)).k)
      .toBeCloseTo(before.k, 2);
  });

  test("Fit restores framing that shows every visible node", async ({
    page,
  }) => {
    await openGraph(page);
    const fitted = await view(page);
    for (let i = 0; i < 4; i++) {
      await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    }
    await page.getByRole("button", { name: "Fit", exact: true }).click();
    await expect
      .poll(async () => (await view(page)).k)
      .toBeCloseTo(fitted.k, 2);
    await page.locator(".graph-scroll").scrollIntoViewIfNeeded();
    const stage = (await page.locator(".graph-scroll").boundingBox())!;
    for (const handle of await visibleNodes(page).all()) {
      const c = await center(handle);
      expect(c.x).toBeGreaterThanOrEqual(stage.x);
      expect(c.x).toBeLessThanOrEqual(stage.x + stage.width);
      expect(c.y).toBeGreaterThanOrEqual(stage.y);
      expect(c.y).toBeLessThanOrEqual(stage.y + stage.height);
    }
  });
});

test.describe("dragging nodes", () => {
  test("moves the node, keeps edges attached and survives Reset layout", async ({
    page,
  }) => {
    await openGraph(page);
    const arch = node(page, ARCH);
    const original = await position(arch);
    const screenBefore = await center(arch);
    // Edges that touch the dragged node.
    const incident = await page
      .locator(
        `path.edge[data-source="${ARCH}"]:not(.is-off), path.edge[data-target="${ARCH}"]:not(.is-off)`,
      )
      .evaluateAll((edges) =>
        edges.map((edge) => [
          edge.getAttribute("data-source") as string,
          edge.getAttribute("data-target") as string,
        ]),
      );
    expect(incident.length).toBeGreaterThan(2);

    await page.mouse.move(screenBefore.x, screenBefore.y);
    await page.mouse.down();
    await page.mouse.move(screenBefore.x + 120, screenBefore.y - 80, {
      steps: 12,
    });
    await page.mouse.up();

    const moved = await position(arch);
    expect(distance(moved, original)).toBeGreaterThan(60);
    const screenAfter = await center(arch);
    expect(screenAfter.x).toBeGreaterThan(screenBefore.x + 60);
    expect(screenAfter.y).toBeLessThan(screenBefore.y - 40);

    // Wait for the simulation to settle, then every incident edge must still touch the node.
    await page.waitForTimeout(1200);
    const settled = await position(arch);
    const radius = Number(
      await arch.evaluate(
        (el) => el.querySelector("circle")?.getAttribute("r") ?? "25",
      ),
    );
    for (const [source, target] of incident) {
      const ends = await edgeEnds(page, source as string, target as string);
      const end = source === ARCH ? ends.from : ends.to;
      const gap = distance(end, settled);
      expect(gap, `${source} → ${target}`).toBeLessThan(radius + 8);
    }
    await expect(arch).toHaveClass(/is-pinned/);
    // A drag is not a selection.
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(0);

    await page.getByRole("button", { name: "Reset layout" }).click();
    await expect
      .poll(async () => distance(await position(arch), original), {
        timeout: 5000,
      })
      .toBeLessThan(1.5);
    await expect(arch).not.toHaveClass(/is-pinned/);
  });

  test("dragging a selected node keeps the inspector and focus on it", async ({
    page,
  }) => {
    await openGraph(page);
    const auth = node(page, AUTH);
    const at = await center(auth);
    await page.mouse.click(at.x, at.y);
    await expect(inspector(page).getByRole("heading", { level: 2 })).toHaveText(
      "Authentication",
    );
    await page.getByRole("button", { name: "Focus neighbors" }).click();
    const focused = await visibleNodes(page).count();
    expect(focused).toBeGreaterThan(1);

    const now = await center(auth);
    await page.mouse.move(now.x, now.y);
    await page.mouse.down();
    await page.mouse.move(now.x + 70, now.y + 50, { steps: 8 });
    await page.mouse.up();

    await expect(inspector(page).getByRole("heading", { level: 2 })).toHaveText(
      "Authentication",
    );
    await expect(auth).toHaveClass(/is-selected/);
    await expect(
      page.getByRole("button", { name: "Focus neighbors" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(await visibleNodes(page).count()).toBe(focused);
  });
});

test.describe("filters", () => {
  const kinds = [
    ["documentation", "kind-documentation"],
    ["agent_instructions", "kind-agent_instructions"],
    ["skill", "kind-skill"],
  ] as const;

  for (const [kind, className] of kinds) {
    test(`hiding ${kind} removes exactly those nodes`, async ({ page }) => {
      await openGraph(page);
      const total = await visibleNodes(page).count();
      const ofKind = await page
        .locator(`a.graph-node.${className}:not(.is-off)`)
        .count();
      expect(ofKind).toBeGreaterThan(0);
      await page.locator(`[data-filter-kind="${kind}"]`).uncheck();
      await expect(
        page.locator(`a.graph-node.${className}:not(.is-off)`),
      ).toHaveCount(0);
      await expect(visibleNodes(page)).toHaveCount(total - ofKind);
      // Edges never dangle toward hidden nodes.
      const hiddenIds = await page
        .locator(`a.graph-node.${className}`)
        .evaluateAll((els) => els.map((e) => e.getAttribute("data-node")));
      for (const id of hiddenIds) {
        await expect(
          page.locator(
            `path.edge[data-source="${id}"]:not(.is-off), path.edge[data-target="${id}"]:not(.is-off)`,
          ),
        ).toHaveCount(0);
      }
      await page.locator(`[data-filter-kind="${kind}"]`).check();
      await expect(visibleNodes(page)).toHaveCount(total);
    });
  }

  test("repository references are off by default and can be shown", async ({
    page,
  }) => {
    await openGraph(page);
    const files = page.locator(
      "a.graph-node.kind-repository_file, g.graph-node.kind-repository_file",
    );
    await expect(
      page.locator("svg.graph .graph-node.kind-repository_file:not(.is-off)"),
    ).toHaveCount(0);
    await page.locator('[data-filter-kind="repository_file"]').check();
    expect(
      await page
        .locator("svg.graph .graph-node.kind-repository_file:not(.is-off)")
        .count(),
    ).toBeGreaterThan(0);
    void files;
  });

  test("a knowledge-role filter hides only that role", async ({ page }) => {
    await openGraph(page);
    const decisions = page.locator("a.graph-node.role-decision");
    const count = await decisions.count();
    expect(count).toBeGreaterThan(0);
    const total = await visibleNodes(page).count();
    await page.locator('[data-filter-role="decision"]').uncheck();
    await expect(
      page.locator("a.graph-node.role-decision:not(.is-off)"),
    ).toHaveCount(0);
    await expect(visibleNodes(page)).toHaveCount(total - count);
    await expect(node(page, ARCH)).toBeVisible();
    // Role filters never hide agent-facing artifacts.
    await expect(
      page.locator("a.graph-node.kind-skill:not(.is-off)").first(),
    ).toBeVisible();
  });

  test("hiding the selected node's kind clears the selection", async ({
    page,
  }) => {
    await openGraph(page);
    const at = await center(node(page, AUTH));
    await page.mouse.click(at.x, at.y);
    await expect(node(page, AUTH)).toHaveClass(/is-selected/);
    await page.locator('[data-filter-kind="documentation"]').uncheck();
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(0);
    await expect(inspector(page)).toContainText("Select a node");
  });
});

test.describe("focus and deep links", () => {
  test("?focus selects the node and frames its neighbourhood", async ({
    page,
  }) => {
    await openGraph(page, `?focus=${encodeURIComponent(AUTH)}`);
    const auth = node(page, AUTH);
    await expect(auth).toHaveClass(/is-selected/);
    await expect(inspector(page).getByRole("heading", { level: 2 })).toHaveText(
      "Authentication",
    );
    await expect(inspector(page)).toContainText("Source");
    const stage = (await page.locator(".graph-scroll").boundingBox())!;
    const c = await center(auth);
    expect(c.x).toBeGreaterThan(stage.x);
    expect(c.x).toBeLessThan(stage.x + stage.width);
    // Neighbours are emphasised, the rest faded rather than removed.
    const neighbours = await page.locator("a.graph-node.is-neighbor").count();
    expect(neighbours).toBeGreaterThan(1);
    const faded = page
      .locator(
        "svg.graph.has-selection a.graph-node:not(.is-selected):not(.is-neighbor):not(.is-off)",
      )
      .first();
    await expect(faded).toHaveCSS("opacity", "0.18");
    await expect(page.locator("a.graph-node.is-neighbor").first()).toHaveCSS(
      "opacity",
      "1",
    );
  });

  test("?focus accepts a repository path and unknown ids are ignored", async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await openGraph(page, "?focus=specs/authentication.md");
    await expect(node(page, AUTH)).toHaveClass(/is-selected/);
    await openGraph(page, "?focus=does-not-exist.md");
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(0);
    expect(errors()).toEqual([]);
  });

  test("?focus on a hidden kind reveals it", async ({ page }) => {
    await openGraph(page, `?focus=${encodeURIComponent("repo:AGENTS.md")}`);
    await expect(node(page, "repo:AGENTS.md")).toHaveClass(/is-selected/);
    await expect(inspector(page).getByRole("heading", { level: 2 })).toHaveText(
      "AGENTS.md",
    );
  });

  test("selecting by click updates the URL and focus mode hides unrelated nodes", async ({
    page,
  }) => {
    await openGraph(page);
    const at = await center(node(page, ARCH));
    await page.mouse.click(at.x, at.y);
    await expect(page).toHaveURL(/focus=architecture\.md/);
    const all = await visibleNodes(page).count();
    await page.getByRole("button", { name: "Focus neighbors" }).click();
    const focused = await visibleNodes(page).count();
    expect(focused).toBeLessThan(all);
    expect(focused).toBeGreaterThan(1);
    await expect(node(page, ARCH)).toBeVisible();
    await page.locator(".graph-scroll").focus();
    await page.keyboard.press("Escape");
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(0);
    await expect(visibleNodes(page)).toHaveCount(all);
    await expect(page).not.toHaveURL(/focus=/);
  });

  test("the inspector opens the selected page", async ({ page }) => {
    await openGraph(page, `?focus=${encodeURIComponent(AUTH)}`);
    await inspector(page).getByRole("link", { name: "Open page" }).click();
    await expect(page).toHaveURL(/\/specs\/authentication\/$/);
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Authentication",
    );
  });

  test("double-clicking a node opens its page", async ({ page }) => {
    await openGraph(page);
    const at = await center(node(page, AUTH));
    await page.mouse.dblclick(at.x, at.y);
    await expect(page).toHaveURL(/\/specs\/authentication\/$/);
  });

  test("neighbour buttons in the inspector move the selection", async ({
    page,
  }) => {
    await openGraph(page, `?focus=${encodeURIComponent(ARCH)}`);
    await inspector(page).locator(".graph-neighbors button").first().click();
    await expect(page.locator("a.graph-node.is-selected")).not.toHaveAttribute(
      "data-node",
      ARCH,
    );
    await expect(page.locator("a.graph-node.is-selected")).toHaveCount(1);
  });
});
