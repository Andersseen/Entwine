/** Exercise a real published package in an isolated consumer, on any supported OS. */
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({ options: { version: { type: "string" } } });
const version = values.version;
if (!version || !/^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$/.test(version)) {
  throw new Error("--version must identify a published release");
}
const temporary = mkdtempSync(join(tmpdir(), "entwine-registry-"));
try {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm";
  execFileSync(
    npm,
    ["install", "--prefix", temporary, `@entwine/cli@${version}`],
    {
      stdio: "inherit",
      shell: process.platform === "win32",
    },
  );
  const consumer = join(temporary, "consumer");
  mkdirSync(consumer);
  const launcher = join(
    temporary,
    "node_modules/@entwine/cli/dist/launcher.js",
  );
  for (const args of [
    ["--version"],
    ["init", "--yes"],
    ["check"],
    ["build"],
    ["context", "--json"],
    ["setup", "--dry-run"],
  ]) {
    execFileSync(process.execPath, [launcher, ...args], {
      cwd: consumer,
      stdio: "inherit",
    });
  }
  console.log(`✓ public @entwine/cli@${version} consumer smoke passed`);
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
