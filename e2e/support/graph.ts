import { expect, type Locator, type Page } from "@playwright/test";

export const graphPath = "/__entwine/graph/";

export interface Point {
  x: number;
  y: number;
}

/** Open the graph page and wait until the script has taken it over. */
export async function openGraph(page: Page, query = ""): Promise<void> {
  await page.goto(`${graphPath}${query}`);
  await expect(page.locator("#graph-app.is-live")).toBeVisible();
}

export const node = (page: Page, id: string): Locator =>
  page.locator(`a.graph-node[data-node="${id}"]`);

export const inspector = (page: Page): Locator =>
  page.locator(".graph-inspector");

/** Graph-space position of a node, from its SVG transform. */
export async function position(locator: Locator): Promise<Point> {
  const transform = (await locator.getAttribute("transform")) ?? "";
  const match = /translate\(\s*(-?[\d.]+)[ ,]\s*(-?[\d.]+)\s*\)/.exec(
    transform,
  );
  if (!match) throw new Error(`No translate in "${transform}"`);
  return { x: Number(match[1]), y: Number(match[2]) };
}

/** The viewport transform {x, y, k}. */
export async function view(
  page: Page,
): Promise<{ x: number; y: number; k: number }> {
  const transform =
    (await page.locator("svg.graph .viewport").getAttribute("transform")) ?? "";
  const match =
    /translate\(\s*(-?[\d.]+)\s+(-?[\d.]+)\)\s*scale\(\s*([\d.]+)\)/.exec(
      transform,
    );
  if (!match) throw new Error(`No viewport transform in "${transform}"`);
  return { x: Number(match[1]), y: Number(match[2]), k: Number(match[3]) };
}

/** Centre of the node's shape on screen. */
export async function center(locator: Locator): Promise<Point> {
  await locator.scrollIntoViewIfNeeded();
  const box = await locator.locator(".shape").boundingBox();
  if (!box) throw new Error("Node is not rendered");
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

/** Start and end points of an edge's path, in graph space. */
export async function edgeEnds(
  page: Page,
  source: string,
  target: string,
): Promise<{ from: Point; to: Point }> {
  const d =
    (await page
      .locator(`path.edge[data-source="${source}"][data-target="${target}"]`)
      .getAttribute("d")) ?? "";
  const numbers = (d.match(/-?\d+(\.\d+)?/g) ?? []).map(Number);
  if (numbers.length < 4) throw new Error(`Unparseable edge path "${d}"`);
  return {
    from: { x: numbers[0] as number, y: numbers[1] as number },
    to: {
      x: numbers[numbers.length - 2] as number,
      y: numbers[numbers.length - 1] as number,
    },
  };
}

export const distance = (a: Point, b: Point): number =>
  Math.hypot(a.x - b.x, a.y - b.y);

export const visibleNodes = (page: Page): Locator =>
  page.locator("a.graph-node:not(.is-off)");

/** Fail the test on console errors and uncaught exceptions. */
export function trackErrors(page: Page): () => string[] {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  return () => errors;
}
