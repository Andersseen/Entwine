/**
 * Progressive enhancement for the homepage. The page is complete HTML without this
 * script: copy buttons are `hidden` until it runs, and the Web Components are only
 * loaded once the agent section is about to be seen.
 */
import { initMotion } from "@andersseen/motion";
// Utility classes the components put on their host elements (the Tailwind reset in
// this file is stripped at build time, see astro.config.ts). Render-blocking by design:
// a late stylesheet would reflow the tabs when they upgrade.
import "@andersseen/web-components/style.css";

const status = document.querySelector<HTMLElement>("#copy-status");
let resetTimer: number | undefined;

async function writeClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Insecure contexts and some embedded browsers: fall back to a selection copy.
    const field = document.createElement("textarea");
    field.value = text;
    field.setAttribute("readonly", "");
    field.style.position = "fixed";
    field.style.opacity = "0";
    document.body.append(field);
    field.select();
    const copied = document.execCommand("copy");
    field.remove();
    return copied;
  }
}

function wireCopyButtons(): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>(
    "button[data-copy-target]",
  )) {
    const source = document.getElementById(button.dataset.copyTarget ?? "");
    if (!source) continue;
    const label = button.querySelector<HTMLElement>(".copy-label");
    const icon = button.querySelector<HTMLElement>("[and-icon]");
    button.hidden = false;
    button.addEventListener("click", async () => {
      const copied = await writeClipboard(source.textContent?.trim() ?? "");
      const name = button.dataset.copyName ?? "command";
      if (label) label.textContent = copied ? "Copied" : "Copy failed";
      icon?.setAttribute("and-icon", copied ? "check" : "copy");
      if (status) {
        status.textContent = copied
          ? `Copied ${name} to the clipboard.`
          : `Could not copy the ${name}. Select it and copy manually.`;
      }
      window.clearTimeout(resetTimer);
      resetTimer = window.setTimeout(() => {
        if (label) label.textContent = "Copy";
        icon?.setAttribute("and-icon", "copy");
        if (status) status.textContent = "";
      }, 2400);
    });
  }
}

const components = [
  "and-tabs",
  "and-tabs-list",
  "and-tabs-trigger",
  "and-tabs-content",
  "and-badge",
] as const;

type Ready = HTMLElement & { componentOnReady?: () => Promise<unknown> };

/**
 * @andersseen/web-components@0.5.1 upgrades a tab group after its panels have been
 * rendered, so no panel starts selected when the elements are defined late (after the
 * page is parsed). Re-assigning the same value once everything is ready selects it.
 */
async function selectInitialTab(): Promise<void> {
  for (const tabs of document.querySelectorAll<Ready>("and-tabs")) {
    const parts = tabs.querySelectorAll<Ready>(
      "and-tabs-list, and-tabs-trigger, and-tabs-content",
    );
    // Bounded: a missing element must not keep the rest of the page from loading.
    await Promise.race([
      Promise.all([tabs, ...parts].map((part) => part.componentOnReady?.())),
      new Promise((resolve) => setTimeout(resolve, 2000)),
    ]);
    const initial = tabs.getAttribute("default-value") ?? "";
    Object.assign(tabs, { value: "" });
    await new Promise((resolve) => setTimeout(resolve, 50));
    Object.assign(tabs, { value: initial });
  }
}

let componentsLoaded = false;
async function loadComponents(): Promise<void> {
  if (componentsLoaded) return;
  componentsLoaded = true;
  // Static specifiers keep Vite able to bundle exactly these nine elements.
  const modules = await Promise.all([
    import("@andersseen/web-components/components/and-tabs.js"),
    import("@andersseen/web-components/components/and-tabs-list.js"),
    import("@andersseen/web-components/components/and-tabs-trigger.js"),
    import("@andersseen/web-components/components/and-tabs-content.js"),
    import("@andersseen/web-components/components/and-badge.js"),
  ]);
  for (const module of modules) module.defineCustomElement();
  await selectInitialTab();
  document.documentElement.dataset.components = components.join(" ");
}

function whenNearViewport(selector: string, load: () => void): void {
  const target = document.querySelector(selector);
  if (!target || !("IntersectionObserver" in window)) {
    load();
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        observer.disconnect();
        load();
      }
    },
    { rootMargin: "800px 0px" },
  );
  observer.observe(target);
}

wireCopyButtons();
whenNearViewport("#integrate", () => void loadComponents());
// Elements marked [and-motion] fade in once; reduced motion is honoured by the library.
whenNearViewport("[and-motion]", async () => {
  initMotion({ once: true });
});
