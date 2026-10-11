/**
 * External packaged-consumer smoke for @entwine/mcp and the skills kit.
 *
 * Stages and packs this commit's @entwine/cli, platform binary and @entwine/mcp,
 * installs them with npm into a project OUTSIDE the monorepo (plus the MCP SDK
 * client from the registry), installs the skills with the `skills` CLI, then
 * drives the installed server with a real MCP client and no ENTWINE_BIN.
 *
 * Usage: smoke-mcp.ts --binary <path-to-entwine>
 */
import { execFileSync } from "node:child_process";
import {
  existsSync,
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
import { stageMain, stageMcp, stagePlatform } from "./stage-npm.ts";
import { binaryPath, hostTarget, platformPackage } from "./targets.ts";

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
const sdk = (
  JSON.parse(readFileSync(join(root, "packages/mcp/package.json"), "utf8")) as {
    devDependencies: Record<string, string>;
  }
).devDependencies["@modelcontextprotocol/client"];
const target = hostTarget();
const work = mkdtempSync(join(tmpdir(), "entwine-mcp-smoke-"));

const consumerScript = `
import assert from "node:assert/strict";
import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

const [server, repo] = process.argv.slice(2);
const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => k !== "ENTWINE_BIN"));
const client = new Client({ name: "external-consumer", version: "0" });
await client.connect(new StdioClientTransport({ command: process.execPath, args: [server, "--root", repo], env, stderr: "inherit" }));
const tools = (await client.listTools()).tools.map((t) => t.name);
assert.equal(tools.length, 7);
const overview = (await client.callTool({ name: "entwine_project_overview", arguments: {} })).structuredContent;
assert.equal(overview.schema_version, "0.4");
assert.equal(overview.cli_version, process.env.EXPECTED_VERSION);
const list = (await client.callTool({ name: "entwine_list_documents", arguments: {} })).structuredContent;
assert.ok(list.documents.some((d) => d.id === "index.md"));
const doc = await client.readResource({ uri: "entwine://document/index.md" });
assert.match(doc.contents[0].text, /Smoke project/);
const check = (await client.callTool({ name: "entwine_check", arguments: {} })).structuredContent;
assert.equal(check.passed, true);
const setup = (await client.callTool({ name: "entwine_setup_preview", arguments: { provider: "github" } })).structuredContent;
assert.equal(setup.changed_files, false);
assert.equal((await client.listPrompts()).prompts.length, 3);
await client.close();
console.log("external consumer: ok");
`;

try {
  stagePlatform(
    target,
    resolve(binaryPath(values.binary)),
    version,
    join(work, "platform"),
  );
  stageMain(version, join(work, "main"));
  stageMcp(version, join(work, "mcp"));
  const pack = (dir: string): string =>
    join(dir, run(dir, "pack", "--silent").trim().split("\n").pop() ?? "");
  const platformTarball = pack(join(work, "platform"));
  const mainPath = join(work, "main/package.json");
  const main = JSON.parse(readFileSync(mainPath, "utf8")) as Record<
    string,
    unknown
  >;
  // Only the host platform tarball exists locally.
  main.optionalDependencies = { [platformPackage(target)]: version };
  writeFileSync(mainPath, `${JSON.stringify(main, null, 2)}\n`);
  const mainTarball = pack(join(work, "main"));
  const mcpTarball = pack(join(work, "mcp"));

  const project = join(work, "consumer");
  mkdirSync(project);
  writeFileSync(
    join(project, "package.json"),
    JSON.stringify({
      private: true,
      type: "module",
      dependencies: {
        "@entwine/mcp": `file:${mcpTarball}`,
        "@modelcontextprotocol/client": sdk,
      },
      overrides: {
        "@entwine/cli": `file:${mainTarball}`,
        [platformPackage(target)]: `file:${platformTarball}`,
      },
    }),
  );
  run(project, "install", "--no-audit", "--no-fund");
  const server = join(project, "node_modules/@entwine/mcp/dist/cli.js");
  if (!existsSync(server)) throw new Error(`Missing ${server}`);

  // A repository fixture with its own user-authored files.
  const repo = join(work, "repo");
  mkdirSync(join(repo, "docs"), { recursive: true });
  writeFileSync(join(repo, "docs/index.md"), "# Smoke project\n\nHello.\n");
  writeFileSync(join(repo, "AGENTS.md"), "# Mine\n");
  execFileSync("git", ["init", "-q"], { cwd: repo });

  // Skills: install with the `skills` CLI, untouched user files stay.
  const skillsCli = join(root, "node_modules/skills/bin/cli.mjs");
  const skillNames = [
    "entwine-get-started",
    "entwine-knowledge",
    "entwine-deployment",
  ];
  execFileSync(
    process.execPath,
    [
      skillsCli,
      "add",
      join(root, "skills"),
      "--skill",
      ...skillNames,
      "--agent",
      "codex",
      "claude-code",
      "--copy",
      "-y",
    ],
    {
      cwd: repo,
      stdio: "inherit",
      env: { ...process.env, DISABLE_TELEMETRY: "1", DO_NOT_TRACK: "1" },
    },
  );
  for (const name of skillNames)
    for (const dir of [".agents/skills", ".claude/skills"])
      if (!existsSync(join(repo, dir, name, "SKILL.md")))
        throw new Error(`Skill not installed at ${dir}/${name}`);
  if (readFileSync(join(repo, "AGENTS.md"), "utf8") !== "# Mine\n")
    throw new Error("Skill installation modified a user-authored AGENTS.md");

  writeFileSync(join(project, "client.mjs"), consumerScript);
  const output = execFileSync(process.execPath, ["client.mjs", server, repo], {
    cwd: project,
    encoding: "utf8",
    env: { ...process.env, EXPECTED_VERSION: version },
  });
  if (!output.includes("external consumer: ok")) throw new Error(output);
  if (existsSync(join(repo, "dist")) || existsSync(join(repo, ".github")))
    throw new Error("Read-only/preview tools changed the repository");
  console.log(
    `✓ packaged @entwine/mcp works from outside the monorepo on ${target.os}-${target.cpu}`,
  );
} finally {
  rmSync(work, { recursive: true, force: true });
}
