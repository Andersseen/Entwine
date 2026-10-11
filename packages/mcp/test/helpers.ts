import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

export const repoRoot = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
export const serverEntry = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../dist/cli.js",
);
export const entwineBinary =
  process.env.ENTWINE_BIN ??
  join(
    repoRoot,
    "target/debug",
    process.platform === "win32" ? "entwine.exe" : "entwine",
  );

export function tempDir(prefix = "entwine-mcp-"): string {
  return realpathSync(mkdtempSync(join(tmpdir(), prefix)));
}

export function cleanup(...dirs: string[]): void {
  for (const dir of dirs) rmSync(dir, { recursive: true, force: true });
}

export function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8" });
}

/** Run the real CLI directly, for comparison with what the MCP returned. */
export function cli(args: string[]): string {
  return execFileSync(entwineBinary, args, {
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1" },
  });
}

/** A committed repository: the kitchen-sink example plus a nested doc and a spaced file name. */
export function representativeRepo(): string {
  const dir = tempDir();
  cpSync(join(repoRoot, "examples/kitchen-sink"), dir, { recursive: true });
  rmSync(join(dir, "dist"), { recursive: true, force: true });
  mkdirSync(join(dir, "docs/guides/deep"), { recursive: true });
  writeFileSync(
    join(dir, "docs/guides/deep/nested.md"),
    "# Nested guide\n\nBack to the [index](../../index.md).\n",
  );
  writeFileSync(
    join(dir, "docs/my notes.md"),
    "# Notes with a space\n\nSee [architecture](architecture.md).\n",
  );
  git(dir, "init", "-q");
  git(dir, "remote", "add", "origin", "https://github.com/acme/harbor.git");
  git(dir, "add", "-A");
  git(
    dir,
    "-c",
    "user.name=t",
    "-c",
    "user.email=t@example.com",
    "commit",
    "-qm",
    "fixture",
  );
  return dir;
}

export function gitStatus(dir: string): string {
  return git(dir, "status", "--porcelain", "--ignored");
}

/** A project that has a git remote but no docs/ at all. */
export function bareRepo(): string {
  const dir = tempDir();
  git(dir, "init", "-q");
  git(dir, "remote", "add", "origin", "https://github.com/acme/bare.git");
  return dir;
}

export interface Connected {
  client: Client;
  close: () => Promise<void>;
  stderr: () => string;
}

export async function connect(
  root: string,
  env: Record<string, string> = {},
  args: string[] = ["--root", root],
): Promise<Connected> {
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [serverEntry, ...args],
    env: {
      ...(Object.fromEntries(
        Object.entries(process.env).filter(
          (e): e is [string, string] => e[1] !== undefined,
        ),
      ) as Record<string, string>),
      ENTWINE_BIN: entwineBinary,
      ...env,
    },
    stderr: "pipe",
  });
  let captured = "";
  transport.stderr?.on("data", (chunk: Buffer) => {
    captured += chunk.toString();
  });
  const client = new Client({ name: "entwine-test", version: "0.0.0" });
  await client.connect(transport);
  return { client, close: () => client.close(), stderr: () => captured };
}

export function text(result: unknown): string {
  const content = (result as { content?: { type: string; text?: string }[] })
    .content;
  return (content ?? []).map((c) => c.text ?? "").join("\n");
}

export function structured<T = Record<string, unknown>>(result: unknown): T {
  return (result as { structuredContent: T }).structuredContent;
}

/** Write a stand-in "entwine" Node script, used to simulate CLI faults. */
export function fakeCli(dir: string, body: string): string {
  const path = join(dir, "fake-entwine.js");
  writeFileSync(path, body);
  return path;
}

export function exists(path: string): boolean {
  return existsSync(path);
}

/** Text of the first text content of a resource read. */
export function resourceText(result: { contents: object[] }): string {
  return (result.contents[0] as { text?: string } | undefined)?.text ?? "";
}

export function promptText(result: {
  messages: { content: unknown }[];
}): string {
  const content = result.messages[0]?.content as { text?: string } | undefined;
  return content?.text ?? "";
}
