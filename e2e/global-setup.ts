/**
 * Build real Entwine output with the real CLI, then serve it.
 *   full    – the kitchen-sink with agent knowledge published
 *   private – the same project with discovery on but publication off, plus canaries
 */
import { execFileSync } from "node:child_process";
import {
  appendFileSync,
  cpSync,
  existsSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import type { Server } from "node:http";
import { join, resolve } from "node:path";
import { canary, fixtureRoot, ports, work } from "./support/fixtures.ts";
import { serve } from "./support/static-server.ts";

const repository = resolve(import.meta.dirname, "..");
const binary =
  process.env.ENTWINE_BIN ??
  join(
    repository,
    "target/debug",
    process.platform === "win32" ? "entwine.exe" : "entwine",
  );

function stage(name: "full" | "private"): string {
  const directory = fixtureRoot(name);
  cpSync(join(repository, "examples/kitchen-sink"), directory, {
    recursive: true,
    filter: (source) => !/[\\/]dist([\\/]|$)/.test(source),
  });
  return directory;
}

/** A Git checkout with an origin lets Entwine emit "Source" links, like a real project. */
function build(directory: string): void {
  const git = (...args: string[]) =>
    execFileSync(
      "git",
      ["-c", "user.name=E2E", "-c", "user.email=e2e@example.invalid", ...args],
      { cwd: directory, stdio: "pipe" },
    );
  git("init", "-q", "-b", "main");
  git("remote", "add", "origin", "https://github.com/example/harbor.git");
  git("add", "-A");
  git("commit", "-q", "-m", "fixture", "--no-gpg-sign");
  execFileSync(binary, ["build", directory], { stdio: "pipe" });
}

export default async function setup(): Promise<() => Promise<void>> {
  if (!existsSync(binary)) {
    throw new Error(
      `Build the CLI first: cargo build -p entwine-cli (${binary})`,
    );
  }
  rmSync(work, { recursive: true, force: true });

  build(stage("full"));

  const hidden = stage("private");
  const config = join(hidden, "entwine.toml");
  const text = readFileSync(config, "utf8");
  if (!text.includes("include_agent_knowledge = true")) {
    throw new Error("kitchen-sink no longer publishes agent knowledge");
  }
  writeFileSync(
    config,
    text.replace(
      "include_agent_knowledge = true",
      "include_agent_knowledge = false",
    ),
  );
  for (const file of [
    "AGENTS.md",
    ".agents/skills/export-catalogue/SKILL.md",
    ".agents/skills/export-catalogue/references/fields.md",
    "services/catalogue-api/AGENTS.md",
  ]) {
    appendFileSync(join(hidden, file), `\n${canary}\n`);
  }
  build(hidden);

  const servers: Server[] = [
    await serve(join(fixtureRoot("full"), "dist"), ports.full),
    await serve(join(hidden, "dist"), ports.private),
  ];
  return async () => {
    await Promise.all(
      servers.map(
        (server) => new Promise<void>((done) => server.close(() => done())),
      ),
    );
    rmSync(work, { recursive: true, force: true });
  };
}
