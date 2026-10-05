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

export default defineConfig({ output: "static", integrations: [showcase] });
