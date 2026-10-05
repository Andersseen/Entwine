#!/usr/bin/env node
// Thin launcher: the real CLI is a native binary shipped in a per-platform
// package (@entwine/cli-<platform>-<arch>) selected by npm via `os`/`cpu`.
import { spawnSync } from "node:child_process";
import { chmodSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const supported = ["darwin-arm64", "darwin-x64", "linux-x64", "win32-x64"];
const key = `${process.platform}-${process.arch}`;

function fail(message: string): never {
  console.error(`entwine: ${message}`);
  process.exit(1);
}

if (!supported.includes(key)) {
  fail(
    `no prebuilt binary for ${key}. Supported: ${supported.join(", ")}. ` +
      "Build from source: https://github.com/Andersseen/Entwine",
  );
}

const packageName = `@entwine/cli-${key}`;
let binary: string;
try {
  const manifest = createRequire(import.meta.url).resolve(
    `${packageName}/package.json`,
  );
  binary = join(
    dirname(manifest),
    "bin",
    process.platform === "win32" ? "entwine.exe" : "entwine",
  );
} catch {
  fail(
    `${packageName} is not installed. Reinstall without --omit=optional ` +
      "(package managers must install optionalDependencies).",
  );
}

if (process.platform !== "win32") {
  try {
    chmodSync(binary, 0o755);
  } catch {
    // Read-only installs are fine when the packaged mode is already executable.
  }
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) fail(`could not run ${binary}: ${result.error.message}`);
if (result.signal) process.kill(process.pid, result.signal);
process.exit(result.status ?? 1);
