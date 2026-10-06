/**
 * Prove the npm distribution works: stage both packages for the host platform,
 * `npm pack` them, install the tarballs into a clean project, run `entwine`.
 * Usage: smoke-npm.ts --binary <path-to-entwine>
 */
import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { root } from "./paths.ts";
import { stageMain, stagePlatform } from "./stage-npm.ts";
import { hostTarget, platformPackage } from "./targets.ts";

const { values } = parseArgs({ options: { binary: { type: "string" } } });
if (!values.binary) throw new Error("--binary is required");

const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const shell = process.platform === "win32";
const run = (cwd: string, ...args: string[]): string =>
  execFileSync(npm, args, {
    cwd,
    encoding: "utf8",
    shell,
    stdio: ["ignore", "pipe", "inherit"],
  });

const version = (
  JSON.parse(readFileSync(join(root, "packages/cli/package.json"), "utf8")) as {
    version: string;
  }
).version;
const target = hostTarget();
const work = mkdtempSync(join(tmpdir(), "entwine-npm-"));
try {
  stagePlatform(
    target,
    resolve(values.binary),
    version,
    join(work, "platform"),
  );
  stageMain(version, join(work, "main"));
  const pack = (dir: string): string =>
    join(dir, run(dir, "pack", "--silent").trim().split("\n").pop() ?? "");
  const platformTarball = pack(join(work, "platform"));
  // Keep this smoke offline and limited to the host tarball. The separate
  // public-consumer matrix exercises registry distribution on every native OS.
  const manifestPath = join(work, "main/package.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Record<
    string,
    unknown
  >;
  manifest.optionalDependencies = { [platformPackage(target)]: version };
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
  const mainTarball = pack(join(work, "main"));
  const project = join(work, "project");
  mkdirSync(project);
  writeFileSync(
    join(project, "package.json"),
    JSON.stringify({
      private: true,
      dependencies: { "@entwine/cli": `file:${mainTarball}` },
      overrides: { [platformPackage(target)]: `file:${platformTarball}` },
    }),
  );
  run(project, "install", "--no-audit", "--no-fund", "--offline");
  const bin = join(
    project,
    "node_modules/.bin",
    process.platform === "win32" ? "entwine.cmd" : "entwine",
  );
  const output = execFileSync(bin, ["--version"], {
    encoding: "utf8",
    shell,
  }).trim();
  if (!output.includes(version))
    throw new Error(`Expected version ${version}, got ${output}`);
  execFileSync(bin, ["check", join(root, "examples/kitchen-sink")], {
    stdio: "inherit",
    shell,
  });
  console.log(
    `✓ npm distribution works for ${target.os}-${target.cpu}: ${output}`,
  );
} finally {
  rmSync(work, { recursive: true, force: true });
}
