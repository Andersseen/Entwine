import { existsSync, readFileSync, statSync } from "node:fs";
import type { IncomingMessage, ServerResponse } from "node:http";
import { extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import type { AstroIntegration } from "astro";
import { defineConfig } from "astro/config";

// /docs/ and /demo/ are Entwine output composed by `pnpm build:showcase`, not
// Astro pages. While developing the site, serve them from that artifact.
const artifact = resolve(
  fileURLToPath(new URL("../../deployment", import.meta.url)),
);
const types: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".json": "application/json",
};

function serveShowcase(
  request: IncomingMessage,
  response: ServerResponse,
  next: () => void,
): void {
  const path = decodeURIComponent((request.url ?? "/").split("?")[0] ?? "/");
  const mount = ["/docs", "/demo"].find(
    (m) => path === m || path.startsWith(`${m}/`),
  );
  if (!mount) {
    next();
    return;
  }
  if (path === mount) {
    response.writeHead(308, { location: `${mount}/` }).end();
    return;
  }
  if (!existsSync(join(artifact, "index.html"))) {
    response
      .writeHead(404, { "content-type": "text/plain; charset=utf-8" })
      .end("Run `pnpm build:showcase` once to generate /docs/ and /demo/.");
    return;
  }
  let file = normalize(join(artifact, path));
  if (!file.startsWith(artifact + sep)) {
    next();
    return;
  }
  if (existsSync(file) && statSync(file).isDirectory())
    file = join(file, "index.html");
  if (!existsSync(file)) {
    next();
    return;
  }
  response
    .writeHead(200, {
      "content-type": types[extname(file)] ?? "application/octet-stream",
    })
    .end(readFileSync(file));
}

const showcase: AstroIntegration = {
  name: "entwine-showcase",
  hooks: {
    "astro:server:setup": ({ server }) => {
      server.middlewares.use(serveShowcase);
    },
  },
};

// Whitespace between inline elements is meaningful ("the <code>entwine</code> CLI").
// Astro's default HTML compression drops a newline-leading space before an inline tag
// and glues words to code and links, so it stays off.
// @andersseen/web-components@0.5.1 ships its stylesheet with the full Tailwind
// preflight (global resets for body, p, a, headings, lists, buttons…). The components
// put their utility classes on the host elements, so only the utilities are needed.
// Dropping the reset keeps Entwine's own typography untouched. The slice is located by
// marker and the build fails if a package upgrade moves it.
const andersseenStyles = {
  name: "entwine-andersseen-styles",
  enforce: "pre" as const,
  transform(code: string, id: string) {
    if (!id.split("?")[0]?.endsWith("/web-components/web-components.css"))
      return;
    const start = code.indexOf("/*! tailwindcss v");
    const end = code.indexOf(".sr-only{", start);
    if (start === -1 || end === -1) {
      throw new Error(
        "Cannot locate the Tailwind preflight in @andersseen/web-components; review the pinned version.",
      );
    }
    const trimmed =
      code.slice(0, start) +
      '[hidden]:where(:not([hidden="until-found"])){display:none}' +
      code.slice(end);
    return {
      // :host-context() theme switches are meaningless in a document stylesheet and
      // only make the CSS minifier warn.
      code: trimmed.replace(/[^{}]*:host-context\([^{}]*\{[^{}]*\}/g, ""),
      map: null,
    };
  },
};

export default defineConfig({
  output: "static",
  compressHTML: false,
  integrations: [showcase],
  vite: { plugins: [andersseenStyles] },
});
