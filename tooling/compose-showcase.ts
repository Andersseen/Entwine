/**
 * Assemble the Cloudflare Pages artifact from three independent build outputs:
 * the Astro site at `/`, Entwine's own docs at `/docs/`, and the kitchen-sink
 * example at `/demo/`. Nothing is rebuilt here; missing inputs are fatal.
 */
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
} from "node:fs";
import { join } from "node:path";
import { required, root, showcase } from "./paths.ts";

const MARKER = ".entwine-output";

function requireDirectory(label: string, path: string, file: string): void {
  if (!existsSync(path) || !statSync(path).isDirectory()) {
    throw new Error(
      `${label}: expected build output at ${path}. Build it first.`,
    );
  }
  if (!existsSync(join(path, file))) {
    throw new Error(
      `${label}: ${path} has no ${file}; the build looks incomplete.`,
    );
  }
}

function copyOutput(source: string, destination: string): void {
  cpSync(source, destination, {
    recursive: true,
    // Entwine's publication marker is a build detail, not a published file.
    filter: (path) => !path.endsWith(MARKER),
    verbatimSymlinks: true,
    errorOnExist: true,
    force: false,
  });
}

export function compose(): void {
  requireDirectory("Astro site", showcase.site.source, "index.html");
  requireDirectory("Entwine docs", showcase.docs.source, "index.html");
  requireDirectory("Entwine demo", showcase.demo.source, "index.html");
  for (const [label, path] of [
    ["Entwine docs", showcase.docs.source],
    ["Entwine demo", showcase.demo.source],
  ] as const) {
    if (
      !existsSync(join(path, MARKER)) ||
      readFileSync(join(path, MARKER), "utf8").trim() === ""
    ) {
      throw new Error(
        `${label}: ${path} was not produced by \`entwine build\`.`,
      );
    }
  }
  for (const reserved of ["docs", "demo"]) {
    if (existsSync(join(showcase.site.source, reserved))) {
      throw new Error(
        `The Astro site must not publish /${reserved}/; it is reserved for Entwine output.`,
      );
    }
  }
  if (!showcase.output.startsWith(root) || showcase.output === root) {
    throw new Error(
      `Refusing to stage outside the repository: ${showcase.output}`,
    );
  }

  rmSync(showcase.output, { recursive: true, force: true });
  mkdirSync(showcase.output, { recursive: true });
  copyOutput(showcase.site.source, showcase.output);
  copyOutput(showcase.docs.source, join(showcase.output, "docs"));
  copyOutput(showcase.demo.source, join(showcase.output, "demo"));

  const missing = required.filter(
    (file) => !existsSync(join(showcase.output, file)),
  );
  if (missing.length > 0) {
    throw new Error(
      `Showcase artifact is incomplete. Missing: ${missing.join(", ")}`,
    );
  }
  console.log(`✓ composed showcase at ${showcase.output}`);
}

compose();
