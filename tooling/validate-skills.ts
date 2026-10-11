/**
 * Validate the public Agent Skills kit in skills/ and prove it installs.
 *
 *   1. `skills-ref validate` (the Agent Skills reference validator) on every skill.
 *   2. Kit shape: exactly the expected skills, short entrypoints, one-level references.
 *   3. Every `entwine ...` command and flag mentioned in a skill exists in the CLI's --help.
 *   4. Installation with the `skills` CLI into a clean repository: together,
 *      individually, idempotently, without touching user-authored files.
 *
 * Usage: validate-skills.ts --binary <path-to-entwine>
 */
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { root } from "./paths.ts";
import { binaryPath } from "./targets.ts";

export const expectedSkills = [
  "entwine-deployment",
  "entwine-get-started",
  "entwine-knowledge",
] as const;

const skillsDir = join(root, "skills");

export function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? walk(join(dir, entry.name)) : [join(dir, entry.name)],
  );
}

/** Backticked `entwine <command> [--flags]` spans, minus npx prefixes. */
export function commandSpans(markdown: string): string[] {
  const spans: string[] = [];
  for (const line of markdown.split("\n")) {
    for (const match of line.matchAll(/`([^`\n]+)`/g)) {
      const span = (match[1] ?? "")
        .replace(/^npx (--yes )?@entwine\/cli /, "entwine ")
        .trim();
      if (!/^entwine [a-z]/.test(span)) continue;
      // Prose such as "no `entwine publish`" names things that deliberately do not exist.
      if (
        /\b(no|not|never|without)\b/i.test(line) &&
        /publish|deploy/.test(span)
      )
        continue;
      spans.push(span);
    }
  }
  return spans;
}

export function problemsForSpan(
  span: string,
  help: (args: string[]) => string,
): string[] {
  const [, command, ...rest] = span.split(/\s+/);
  if (!command) return [];
  if (command.startsWith("-")) return []; // `entwine --version`
  const top = help([]);
  if (!new RegExp(`^\\s{2}${command}\\s`, "m").test(top))
    return [`unknown command: ${span}`];
  const own = help([command]);
  return rest
    .filter((token) => /^--?[a-zA-Z]/.test(token))
    .map((token) => token.split("=")[0] ?? token)
    .filter((flag) => !own.includes(flag))
    .map((flag) => `unknown flag ${flag} for \`${command}\`: ${span}`);
}

function main(): void {
  const { values } = parseArgs({ options: { binary: { type: "string" } } });
  if (!values.binary) throw new Error("--binary is required");
  const binary = resolve(binaryPath(values.binary));
  const fail: string[] = [];

  // 1 + 2. Reference validator and kit shape.
  const found = readdirSync(skillsDir).sort();
  if (JSON.stringify(found) !== JSON.stringify([...expectedSkills]))
    fail.push(`skills/ contains ${found.join(", ")}`);
  const validator = join(root, "node_modules/skills-ref/dist/cli.js");
  for (const name of expectedSkills) {
    const dir = join(skillsDir, name);
    try {
      execFileSync(process.execPath, [validator, "validate", dir], {
        stdio: "pipe",
      });
    } catch (error) {
      fail.push(
        `skills-ref rejected ${name}: ${(error as { stderr?: Buffer }).stderr}`,
      );
    }
    const entry = readFileSync(join(dir, "SKILL.md"), "utf8");
    if (entry.split("\n").length > 120)
      fail.push(`${name}/SKILL.md is longer than 120 lines`);
    for (const file of walk(dir)) {
      const rel = file.slice(dir.length + 1).replaceAll("\\", "/");
      if (rel !== "SKILL.md" && !/^references\/[^/]+\.md$/.test(rel))
        fail.push(`${name}: unexpected file ${rel}`);
    }
    for (const link of entry.matchAll(/\]\((references\/[^)#]+)/g)) {
      if (!existsSync(join(dir, link[1] ?? "")))
        fail.push(`${name}: broken reference ${link[1]}`);
    }
  }

  // 3. Commands described by the skills must exist.
  const help = (args: string[]): string =>
    execFileSync(binary, [...args, "--help"], { encoding: "utf8" });
  const seen = new Set<string>();
  for (const file of walk(skillsDir).filter((f) => f.endsWith(".md"))) {
    for (const span of commandSpans(readFileSync(file, "utf8"))) {
      if (seen.has(span)) continue;
      seen.add(span);
      fail.push(...problemsForSpan(span, help));
    }
  }

  // 4. Installation with the `skills` CLI.
  const cli = join(root, "node_modules/skills/bin/cli.mjs");
  const env = { ...process.env, DISABLE_TELEMETRY: "1", DO_NOT_TRACK: "1" };
  const skills = (cwd: string, ...args: string[]): string =>
    execFileSync(process.execPath, [cli, ...args], {
      cwd,
      encoding: "utf8",
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
  const work = mkdtempSync(join(tmpdir(), "entwine-skills-"));
  try {
    const repo = join(work, "repo");
    mkdirSync(join(repo, ".claude/skills/own"), { recursive: true });
    execFileSync("git", ["init", "-q"], { cwd: repo });
    writeFileSync(join(repo, "AGENTS.md"), "# Mine\n");
    writeFileSync(join(repo, "CLAUDE.md"), "# Mine too\n");
    writeFileSync(
      join(repo, ".claude/skills/own/SKILL.md"),
      "---\nname: own\ndescription: user skill\n---\n",
    );
    const listed = skills(repo, "add", skillsDir, "--list");
    for (const name of expectedSkills)
      if (!listed.includes(name)) fail.push(`--list does not show ${name}`);

    const targets = [".agents/skills", ".claude/skills"];
    const install = (...names: string[]): void => {
      skills(
        repo,
        "add",
        skillsDir,
        "--skill",
        ...names,
        "--agent",
        "codex",
        "claude-code",
        "--copy",
        "-y",
      );
    };
    const verify = (names: readonly string[]): void => {
      for (const name of names)
        for (const target of targets) {
          const dir = join(repo, target, name);
          for (const file of walk(join(skillsDir, name))) {
            const rel = file.slice(join(skillsDir, name).length + 1);
            const installed = join(dir, rel);
            if (
              !existsSync(installed) ||
              readFileSync(installed, "utf8") !== readFileSync(file, "utf8")
            )
              fail.push(`${target}/${name}/${rel} missing or different`);
          }
        }
    };
    install("entwine-get-started");
    verify(["entwine-get-started"]);
    if (existsSync(join(repo, ".agents/skills/entwine-knowledge")))
      fail.push("individual install also installed another skill");
    install(...expectedSkills);
    install(...expectedSkills); // idempotent
    verify(expectedSkills);
    const after = (target: string): string[] =>
      readdirSync(join(repo, target)).sort();
    if (after(".agents/skills").join() !== [...expectedSkills].join())
      fail.push(`.agents/skills contains ${after(".agents/skills")}`);
    if (!after(".claude/skills").includes("own"))
      fail.push("user-authored .claude/skills/own was removed");
    if (
      readFileSync(join(repo, "AGENTS.md"), "utf8") !== "# Mine\n" ||
      readFileSync(join(repo, "CLAUDE.md"), "utf8") !== "# Mine too\n"
    )
      fail.push("agent instruction files were modified");
    if (statSync(join(repo, ".claude/skills/own/SKILL.md")).size === 0)
      fail.push("user skill was emptied");
  } finally {
    rmSync(work, { recursive: true, force: true });
  }

  if (fail.length) {
    console.error(`Skills kit problems:\n- ${fail.join("\n- ")}`);
    process.exit(1);
  }
  console.log(
    `✓ ${expectedSkills.length} skills valid and installable; ${seen.size} documented commands exist`,
  );
}

if (process.argv[1]?.endsWith("validate-skills.ts")) main();
