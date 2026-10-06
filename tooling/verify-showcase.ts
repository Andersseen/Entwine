/**
 * Prove the composed artifact works as a static host would serve it: every
 * relative reference under /docs/ and /demo/ must resolve inside the artifact
 * and stay inside its own mount point, and every fragment must exist.
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, posix } from "node:path";
import { required, showcase } from "./paths.ts";

function* walk(directory: string, prefix = ""): Generator<string> {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const relative = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
    if (entry.isDirectory()) yield* walk(join(directory, entry.name), relative);
    else yield relative;
  }
}

const unescapeHtml = (value: string): string =>
  value
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");

function references(html: string): string[] {
  return [...html.matchAll(/\s(?:href|src)="([^"]*)"/g)].map((match) =>
    unescapeHtml(match[1] ?? ""),
  );
}

function resolveTarget(
  artifact: string,
  mount: string,
  page: string,
  reference: string,
): string {
  const withoutFragment = reference.split("#")[0] ?? "";
  const path = withoutFragment.split("?")[0] ?? "";
  const url = posix.join(
    "/",
    mount,
    posix.dirname(page),
    decodeURIComponent(path),
  );
  if (!`${url}/`.startsWith(`/${mount}/`)) {
    throw new Error(`/${mount}/${page}: ${reference} escapes /${mount}/`);
  }
  const file = join(artifact, url);
  if (existsSync(file) && statSync(file).isFile()) return file;
  if (existsSync(join(file, "index.html"))) return join(file, "index.html");
  throw new Error(`/${mount}/${page}: ${reference} resolves to missing ${url}`);
}

export function verify(artifact = showcase.output): void {
  for (const file of required) {
    if (!existsSync(join(artifact, file))) throw new Error(`Missing ${file}`);
  }
  let checked = 0;
  for (const mount of ["docs", "demo"] as const) {
    const base = join(artifact, mount);
    for (const page of [...walk(base)].filter((file) =>
      file.endsWith(".html"),
    )) {
      const html = readFileSync(join(base, page), "utf8");
      for (const reference of references(html)) {
        if (/^(https?:|mailto:)/.test(reference)) continue;
        if (reference.startsWith("/")) {
          throw new Error(
            `/${mount}/${page}: root-absolute reference ${reference}`,
          );
        }
        const target = resolveTarget(artifact, mount, page, reference);
        const fragment = reference.split("#")[1];
        if (
          fragment &&
          !readFileSync(target, "utf8").includes(`id="${fragment}"`)
        ) {
          throw new Error(
            `/${mount}/${page}: #${fragment} is missing from ${target}`,
          );
        }
        checked += 1;
      }
    }
  }
  const landing = readFileSync(join(artifact, "index.html"), "utf8");
  for (const link of [
    'href="/docs/"',
    'href="/demo/"',
    'href="/demo/__entwine/knowledge/"',
    'href="/demo/__entwine/graph/"',
  ]) {
    if (!landing.includes(link))
      throw new Error(`Landing page does not link to ${link}`);
  }
  console.log(
    `✓ verified ${checked} relative references under /docs/ and /demo/`,
  );
}

verify();
