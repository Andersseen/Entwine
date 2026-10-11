/** Fail before building when any release version or workspace lock entry differs. */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./paths.ts";

const cargo = readFileSync(join(root, "Cargo.toml"), "utf8");
const version = cargo.match(
  /\[workspace\.package\]\s+version = "([^"]+)"/,
)?.[1];
if (!version) throw new Error("No Cargo workspace version");
for (const path of [
  "package.json",
  "apps/www/package.json",
  "packages/cli/package.json",
  "packages/mcp/package.json",
]) {
  const manifest = JSON.parse(readFileSync(join(root, path), "utf8")) as {
    version: string;
  };
  if (manifest.version !== version)
    throw new Error(`${path}: ${manifest.version} != ${version}`);
}
const lock = readFileSync(join(root, "Cargo.lock"), "utf8");
for (const name of ["entwine-core", "entwine-engine", "entwine-cli"]) {
  const locked = lock.match(
    new RegExp(`name = "${name}"\\nversion = "([^"]+)"`),
  )?.[1];
  if (locked !== version)
    throw new Error(`Cargo.lock ${name}: ${locked} != ${version}`);
}

// @entwine/mcp ships with exactly the CLI it was built against and only
// interprets the context schema version that CLI emits.
const mcpManifest = JSON.parse(
  readFileSync(join(root, "packages/mcp/package.json"), "utf8"),
) as { dependencies?: Record<string, string> };
const cliDependency = mcpManifest.dependencies?.["@entwine/cli"];
if (cliDependency !== "workspace:*" && cliDependency !== version) {
  throw new Error(
    `packages/mcp must depend on @entwine/cli as workspace:* or exactly ${version}, got ${cliDependency}`,
  );
}
const projections = readFileSync(
  join(root, "crates/entwine-engine/src/projections.rs"),
  "utf8",
);
const schema = projections.match(
  /CONTEXT_SCHEMA_VERSION: &str = "([^"]+)"/,
)?.[1];
if (!schema) throw new Error("No CONTEXT_SCHEMA_VERSION in the engine");
const mcpVersions = readFileSync(
  join(root, "packages/mcp/src/version.ts"),
  "utf8",
);
const supported = mcpVersions.match(
  /SUPPORTED_SCHEMA_VERSIONS: readonly string\[\] = \[([^\]]*)\]/,
)?.[1];
if (!supported?.includes(`"${schema}"`)) {
  throw new Error(
    `@entwine/mcp does not support the engine context schema ${schema} (supports ${supported})`,
  );
}

const expected = process.argv[2];
if (expected && expected !== version) {
  throw new Error(`Release ${expected} != workspace ${version}`);
}
console.log(
  `✓ all package and lock versions agree: ${version} (context schema ${schema})`,
);
