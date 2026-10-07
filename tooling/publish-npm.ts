/**
 * Publish staged npm packages idempotently:
 *   publish-npm.ts <staged-dir>...   (platform packages first, launcher last)
 *
 * A package whose exact version is already public is skipped, so a resumed
 * release never fails because an earlier attempt got partway.
 */
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { lookupVersion } from "./registry.ts";

export type Outcome = "skipped" | "published";

/** Decide and act for one package; `publish` and `exists` are injected for tests. */
export async function publishIfMissing(
  name: string,
  version: string,
  exists: () => Promise<boolean>,
  publish: () => void,
): Promise<Outcome> {
  if (await exists()) return "skipped";
  try {
    publish();
    return "published";
  } catch (error) {
    // A lost response or a concurrent run may have published it anyway.
    if (await exists()) return "skipped";
    throw new Error(
      `npm publish failed for ${name}@${version}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

async function main(): Promise<void> {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm";
  for (const dir of process.argv.slice(2)) {
    const manifest = JSON.parse(
      readFileSync(join(dir, "package.json"), "utf8"),
    ) as { name: string; version: string };
    const outcome = await publishIfMissing(
      manifest.name,
      manifest.version,
      async () =>
        (await lookupVersion(manifest.name, manifest.version)).state ===
        "published",
      () =>
        execFileSync(npm, ["publish", "--access", "public", "--provenance"], {
          cwd: dir,
          stdio: "inherit",
          shell: process.platform === "win32",
        }),
    );
    console.log(
      outcome === "skipped"
        ? `${manifest.name}@${manifest.version} already published; skipping`
        : `✓ published ${manifest.name}@${manifest.version}`,
    );
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error: unknown) => {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  });
}
