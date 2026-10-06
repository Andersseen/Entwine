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
const expected = process.argv[2];
if (expected && expected !== version) {
  throw new Error(`Release ${expected} != workspace ${version}`);
}
console.log(`✓ all package and lock versions agree: ${version}`);
