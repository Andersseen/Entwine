import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** Every input and output of the showcase, in one place. */
export const showcase = {
  /** Staging directory that is uploaded to Cloudflare Pages. */
  output: join(root, "deployment"),
  site: { name: "/", source: join(root, "apps/www/dist") },
  docs: { name: "/docs/", source: join(root, "dist") },
  demo: { name: "/demo/", source: join(root, "examples/kitchen-sink/dist") },
} as const;

/** Files that must exist in the finished artifact. */
export const required = [
  "index.html",
  "docs/index.html",
  "docs/architecture/index.html",
  "docs/state/index.html",
  "docs/roadmap/index.html",
  "docs/__entwine/style.css",
  "docs/__entwine/graph/index.html",
  "docs/__entwine/knowledge/index.html",
  "docs/convention/index.html",
  "docs/deployment/index.html",
  "demo/index.html",
  "demo/architecture/index.html",
  "demo/specs/authentication/index.html",
  "demo/decisions/static-output/index.html",
  "demo/__entwine/style.css",
  "demo/__entwine/graph/index.html",
  "demo/__entwine/knowledge/index.html",
  "demo/design/data-model/index.html",
  "demo/design/index.html",
  "demo/operations/index.html",
] as const;
