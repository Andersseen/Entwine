import { readFileSync } from "node:fs";

/** Context schema versions this release interprets. Anything else is rejected. */
export const SUPPORTED_SCHEMA_VERSIONS: readonly string[] = ["0.4"];

export const PACKAGE_NAME = "@entwine/mcp";

function readVersion(): string {
  const manifest = JSON.parse(
    readFileSync(new URL("../package.json", import.meta.url), "utf8"),
  ) as { version: string };
  return manifest.version;
}

/** Same version as the package; the CLI it ships with is pinned to it exactly. */
export const PACKAGE_VERSION = readVersion();
