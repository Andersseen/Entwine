/**
 * Entwine's own Astro website (apps/www): typography, the agent integration section,
 * the Andersseen components, progressive enhancement, motion and layout.
 */
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { gzipSync } from "node:zlib";
import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";
import { origins } from "./support/fixtures.ts";

test.use({ baseURL: origins.www });

const dist = resolve(import.meta.dirname, "../apps/www/dist");

/** Wait until the lazily loaded Andersseen components are defined and hydrated. */
async function integrate(page: Page): Promise<void> {
  await page.goto("/");
  await page.locator("#integrate").scrollIntoViewIfNeeded();
  // Set by the page script once the elements are defined and the first tab selected.
  await page.locator("html[data-components]").waitFor({ state: "attached" });
  await page.locator("and-tabs.hydrated").waitFor();
}

test.describe("typography", () => {
  test("inline code and links are separated from neighbouring words", async ({
    page,
  }) => {
    await page.goto("/");
    const glued = await page.evaluate(() => {
      const problems: string[] = [];
      const wordChar = /[\p{L}\p{N}]/u;
      const elements = Array.from(
        document.querySelectorAll(
          "main p code, main dd code, main li code, main p a, main dd a",
        ),
      );
      for (const element of elements) {
        const before = element.previousSibling;
        const after = element.nextSibling;
        const tail = before?.textContent?.slice(-1) ?? "";
        const head = after?.textContent?.slice(0, 1) ?? "";
        const label = (element.textContent ?? "").trim().slice(0, 30);
        if (before?.nodeType === Node.TEXT_NODE && wordChar.test(tail))
          problems.push(`before <${element.tagName}> ${label}`);
        if (after?.nodeType === Node.TEXT_NODE && wordChar.test(head))
          problems.push(`after <${element.tagName}> ${label}`);
      }
      return problems;
    });
    expect(glued).toEqual([]);
  });

  test("the reported phrases read correctly in the served HTML", async ({
    page,
  }) => {
    await page.goto("/");
    const text = (await page.locator("main").innerText()).replace(/\s+/g, " ");
    for (const wrong of ["andentwine", "theentwine", "approval.Integration"]) {
      expect(text).not.toContain(wrong);
    }
    expect(text).toContain("approval. Read the integration guide");
  });

  for (const [name, size] of [
    ["desktop", { width: 1280, height: 900 }],
    ["tablet", { width: 820, height: 1100 }],
    ["mobile", { width: 375, height: 812 }],
  ] as const) {
    test(`no horizontal page overflow on ${name}`, async ({
      page,
    }, testInfo) => {
      await page.setViewportSize(size);
      await integrate(page);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth,
      );
      expect(overflow).toBeLessThanOrEqual(0);
      // Long commands scroll inside their own block instead of widening the page.
      const pre = page.locator("#cmd-skills");
      const box = await pre.evaluate((el) => {
        const block = el.closest("pre") as HTMLElement;
        return { scroll: block.scrollWidth, client: block.clientWidth };
      });
      if (name === "mobile") expect(box.scroll).toBeGreaterThan(box.client);
      await page.screenshot({
        path: testInfo.outputPath(`www-${name}.png`),
        fullPage: true,
      });
    });
  }
});

test.describe("agent integration section", () => {
  test("shows three pieces, official npm links and the guide link", async ({
    page,
  }) => {
    await integrate(page);
    const section = page.locator("#integrate");
    for (const [name, href] of [
      ["@entwine/cli", "https://www.npmjs.com/package/@entwine/cli"],
      ["@entwine/mcp", "https://www.npmjs.com/package/@entwine/mcp"],
      ["skills", "https://www.npmjs.com/package/skills"],
    ] as const) {
      await expect(
        section.locator(`a[href="${href}"]`).filter({ hasText: name }),
      ).toHaveCount(1);
    }
    await expect(
      section.getByRole("link", { name: "Read the integration guide" }),
    ).toHaveAttribute("href", "/docs/agent-integration/");
    const outdated = /not (yet )?published|until .* is on npm|unpublished/i;
    expect(await page.locator("main").innerText()).not.toMatch(outdated);
  });

  test("commands are the documented, verified syntax", async ({ page }) => {
    await integrate(page);
    expect(await page.locator("#cmd-skills").innerText()).toBe(
      "npx skills add https://github.com/Andersseen/Entwine/tree/main/skills --agent codex claude-code --copy -y",
    );
    expect(await page.locator("#cmd-codex").innerText()).toBe(
      "codex mcp add entwine -- npx --yes @entwine/mcp --root .",
    );
    expect(await page.locator("#cmd-claude").innerText()).toBe(
      "claude mcp add entwine --scope project -- npx --yes @entwine/mcp --root .",
    );
  });

  test("copy buttons copy exactly the displayed command and announce it", async ({
    page,
    context,
  }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await integrate(page);
    for (const id of ["cmd-skills", "cmd-codex"]) {
      const button = page.locator(`button[data-copy-target="${id}"]`);
      if (id === "cmd-codex") {
        await expect(button).toBeVisible();
      }
      await button.click();
      const copied = await page.evaluate(() => navigator.clipboard.readText());
      expect(copied).toBe((await page.locator(`#${id}`).innerText()).trim());
      await expect(page.locator("#copy-status")).toContainText("Copied");
      await expect(button.locator(".copy-label")).toHaveText("Copied");
      await expect(button.locator(".copy-label")).toHaveText("Copy", {
        timeout: 5000,
      });
    }
  });

  test("tabs expose roles, associate triggers with panels and support the keyboard", async ({
    page,
  }) => {
    await integrate(page);
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveCount(2);
    const codex = page.getByRole("tab", { name: "Codex" });
    const claude = page.getByRole("tab", { name: "Claude Code" });
    await expect(page.getByRole("tablist")).toHaveCount(1);
    await expect(page.getByRole("tabpanel")).toHaveCount(1);
    await expect(page.locator("#cmd-codex")).toBeVisible();
    await expect(page.locator("#cmd-claude")).toBeHidden();
    await codex.focus();
    await page.keyboard.press("ArrowRight");
    await expect(claude).toBeFocused();
    await expect(claude).toHaveAttribute("aria-selected", "true");
    await expect(codex).toHaveAttribute("aria-selected", "false");
    await expect(page.locator("#cmd-claude")).toBeVisible();
    await expect(page.locator("#cmd-codex")).toBeHidden();
    const controls = await claude.getAttribute("aria-controls");
    expect(controls).toBeTruthy();
    await expect(page.locator(`[id="${controls}"]`)).toHaveAttribute(
      "role",
      "tabpanel",
    );
    await page.keyboard.press("Home");
    await expect(codex).toHaveAttribute("aria-selected", "true");
  });

  test("the troubleshooting disclosures open and close with the keyboard", async ({
    page,
  }) => {
    await integrate(page);
    const summary = page.getByText("The server does not start");
    await expect(page.getByText("needs Node 20 or newer")).toBeHidden();
    await summary.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByText("needs Node 20 or newer")).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(page.getByText("needs Node 20 or newer")).toBeHidden();
  });
});

test.describe("progressive enhancement", () => {
  test.use({ javaScriptEnabled: false });

  test("without JavaScript every command is readable and no dead control remains", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.locator("#cmd-skills")).toBeVisible();
    await expect(page.locator("#cmd-codex")).toBeVisible();
    await expect(page.locator("#cmd-claude")).toBeVisible();
    await expect(page.locator("button[data-copy-target]")).toHaveCount(3);
    for (const button of await page.locator("button[data-copy-target]").all()) {
      await expect(button).toBeHidden();
    }
    // Motion never hides content that JavaScript has not revealed.
    for (const piece of await page.locator(".piece").all()) {
      await expect(piece).toHaveCSS("opacity", "1");
    }
    await expect(page.getByRole("tablist")).toHaveCount(0);
  });
});

test.describe("motion", () => {
  test.use({ reducedMotion: "reduce" });

  test("reduced motion keeps everything visible and does not animate", async ({
    page,
  }) => {
    await integrate(page);
    for (const element of await page.locator("[and-motion]").all()) {
      await expect(element).toHaveCSS("opacity", "1");
      const duration = await element.evaluate(
        (node) => getComputedStyle(node).animationDuration,
      );
      expect(["0s", "0.001s"]).toContain(duration);
    }
  });
});

test.describe("accessibility", () => {
  test("axe finds nothing on the homepage, with the components open", async ({
    page,
  }) => {
    await integrate(page);
    await page.getByText("Which agent files become public?").click();
    // Axe samples colours, so let the short entrance animations finish first.
    await expect
      .poll(() =>
        page.evaluate(() =>
          document.getAnimations().every((a) => a.playState !== "running"),
        ),
      )
      .toBe(true);
    const results = await new AxeBuilder({ page }).analyze();
    expect(
      results.violations.map(
        (v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`,
      ),
    ).toEqual([]);
  });

  test("decorative icons are hidden from assistive technology", async ({
    page,
  }) => {
    await page.goto("/");
    const icons = await page.locator("[and-icon]").evaluateAll((nodes) =>
      nodes.map((node) => ({
        hidden: node.getAttribute("aria-hidden") === "true",
        named: node.hasAttribute("aria-label"),
      })),
    );
    expect(icons.length).toBeGreaterThan(5);
    expect(icons.every((icon) => icon.hidden && !icon.named)).toBe(true);
  });

  test("copy buttons have accessible names", async ({ page }) => {
    await integrate(page);
    await expect(
      page.getByRole("button", { name: /Copy\s+skills install command/ }),
    ).toBeVisible();
  });
});

test.describe("bundle budget", () => {
  const html = readFileSync(join(dist, "index.html"), "utf8");
  const asset = (name: string) => readFileSync(join(dist, "_astro", name));

  test("only the small entry script loads eagerly; components are lazy chunks", () => {
    const scripts = [...html.matchAll(/<script[^>]*src="\/_astro\/([^"]+)"/g)];
    expect(scripts).toHaveLength(1);
    const entry = asset(scripts[0]?.[1] ?? "");
    expect(gzipSync(entry).length).toBeLessThan(4 * 1024);
    expect(html).not.toMatch(/modulepreload[^>]*and-(tabs|accordion|badge)/);
    const chunks = readdirSync(join(dist, "_astro")).filter((f) =>
      /^and-.*\.js$/.test(f),
    );
    expect(chunks.length).toBe(5);
  });

  test("the stylesheets carry no Tailwind reset and stay within budget", () => {
    const css = readdirSync(join(dist, "_astro")).filter((f) =>
      f.endsWith(".css"),
    );
    expect(css.length).toBeGreaterThan(0);
    let total = 0;
    for (const file of css) {
      const text = asset(file).toString("utf8");
      expect(text).not.toContain("tailwindcss v");
      expect(text).not.toContain("host-context");
      total += gzipSync(asset(file)).length;
    }
    expect(total).toBeLessThan(32 * 1024);
  });

  test("no unrelated framework runtime is shipped", () => {
    const names = readdirSync(join(dist, "_astro")).join(" ");
    expect(names).not.toMatch(/react|vue|angular|svelte/i);
  });
});
