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
import { probeMcp } from "./mcp-probe.ts";
import { backoffDelay, waitForVersions } from "./registry.ts";
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

const installAttempts = 6;
const installBackoff = { initialMs: 10_000, maxMs: 60_000, factor: 2 };

/** Install both packages into a fresh prefix; returns that prefix. Exits when out of attempts. */
async function installWithRetry(
  parent: string,
  version: string,
): Promise<string> {
  for (let attempt = 1; ; attempt++) {
    const prefix = join(parent, `install-${attempt}`);
    mkdirSync(prefix);
    try {
      execFileSync(
        npm,
        [
          "install",
          "--prefix",
          prefix,
          "--cache",
          join(parent, "cache"),
          "--prefer-online",
          "--no-audit",
          "--no-fund",
          `@entwine/cli@${version}`,
          `@entwine/mcp@${version}`,
        ],
        { stdio: "inherit", shell },
      );
      return prefix;
    } catch {
      if (attempt === installAttempts) {
        console.error(
          `@entwine/cli@${version} and @entwine/mcp@${version} are listed by the registry but could not be installed from a clean cache after ${attempt} attempts.`,
        );
        process.exit(3);
      }
      const delay = backoffDelay(attempt, installBackoff);
      console.log(
        `Install attempt ${attempt} failed; the registry index may still be propagating. Retrying in ${Math.round(delay / 1000)}s.`,
      );
      await new Promise<void>((resolve) => setTimeout(resolve, delay));
    }
  }
}

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
  // The version endpoint can answer before the package index does, so a fresh
  // install may 404 for a few minutes after step 1 passed. Each attempt gets its
  // own prefix so a failed attempt cannot leave partial state behind.
  const installed = await installWithRetry(temporary, version);
  console.log("Step 3/3: consumer smoke");
  const consumer = join(temporary, "consumer");
  mkdirSync(consumer);
  const launcher = join(
    installed,
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
  try {
    const probe = await probeMcp(
      process.execPath,
      [
        join(installed, "node_modules/@entwine/mcp/dist/cli.js"),
        "--root",
        consumer,
      ],
      consumer,
      { ...process.env, ENTWINE_BIN: launcher },
    );
    if (
      probe.serverName !== "entwine" ||
      probe.tools.length < 7 ||
      probe.stray.length
    ) {
      throw new Error(JSON.stringify(probe));
    }
  } catch (error) {
    console.error(
      `@entwine/mcp@${version} installs but does not complete an MCP handshake: ${error instanceof Error ? error.message : error}`,
    );
    process.exit(5);
  }
  console.log(
    `✓ public @entwine/cli@${version} and @entwine/mcp@${version} consumer smoke passed`,
  );
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
