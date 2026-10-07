/**
 * Verify a published release from a clean consumer, in three distinguishable steps:
 *
 *   1. the registry reports every package at the exact version (bounded polling),
 *   2. a fresh, isolated, online install succeeds,
 *   3. the installed CLI works.
 *
 * Failures say which step failed: "not published / not propagated yet" is retryable
 * by re-dispatching the Release workflow, "installs but is broken" is a bad release.
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { waitForVersions } from "./registry.ts";
import { releasePackages } from "./release-state.ts";

const { values } = parseArgs({
  options: {
    version: { type: "string" },
    "timeout-minutes": { type: "string", default: "20" },
  },
});
const version = values.version;
if (!version || !/^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$/.test(version)) {
  throw new Error("--version must identify a published release");
}
const timeoutMs = Number(values["timeout-minutes"]) * 60_000;
if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
  throw new Error("--timeout-minutes must be a positive number");
}

const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const shell = process.platform === "win32";

console.log(`Step 1/3: waiting for the registry to list ${version}`);
const seen = await waitForVersions(releasePackages(), version, {
  timeoutMs,
  backoff: { initialMs: 5_000, maxMs: 60_000, factor: 1.6 },
  log: (message) => console.log(message),
});
if (seen.visible.length !== releasePackages().length) {
  const missing = [...seen.absent, ...seen.unknown];
  console.error(
    [
      `Registry does not list ${missing.join(", ")} at ${version} after ${values["timeout-minutes"]} minutes.`,
      seen.absent.length > 0
        ? `  Registry answered "not found" for: ${seen.absent.join(", ")} — either the publish did not happen or it has not propagated yet.`
        : "",
      seen.unknown.length > 0
        ? `  Registry could not be queried for: ${seen.unknown.join(", ")}.`
        : "",
      "The GitHub release was left as a draft. Re-run: Release → Run workflow → this tag.",
    ]
      .filter(Boolean)
      .join("\n"),
  );
  process.exit(2);
}

const temporary = mkdtempSync(join(tmpdir(), "entwine-registry-"));
try {
  console.log("Step 2/3: fresh isolated install");
  try {
    execFileSync(
      npm,
      [
        "install",
        "--prefix",
        temporary,
        "--cache",
        join(temporary, "cache"),
        "--prefer-online",
        "--no-audit",
        "--no-fund",
        `@entwine/cli@${version}`,
      ],
      { stdio: "inherit", shell },
    );
  } catch {
    console.error(
      `@entwine/cli@${version} is listed by the registry but could not be installed from a clean cache.`,
    );
    process.exit(3);
  }
  console.log("Step 3/3: consumer smoke");
  const consumer = join(temporary, "consumer");
  mkdirSync(consumer);
  const launcher = join(
    temporary,
    "node_modules/@entwine/cli/dist/launcher.js",
  );
  try {
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
  } catch {
    console.error(
      `@entwine/cli@${version} installs but the installed CLI is broken. Do not publish this release.`,
    );
    process.exit(4);
  }
  console.log(`✓ public @entwine/cli@${version} consumer smoke passed`);
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
