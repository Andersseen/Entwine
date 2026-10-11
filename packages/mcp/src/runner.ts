import { execFile } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import type { Config } from "./config.ts";
import { EntwineMcpError } from "./errors.ts";

/** The only subcommands the server may run. There is no free-form execution. */
export type Subcommand = "context" | "check" | "init" | "setup";

export interface CliResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

interface Invocation {
  command: string;
  prefix: string[];
}

const MAX_BUFFER = 256 * 1024 * 1024;

/** Locate the Entwine CLI: ENTWINE_BIN, else the @entwine/cli dependency. */
export function locateCli(config: Pick<Config, "binary">): Invocation {
  if (config.binary) {
    const path = resolve(config.binary);
    if (!existsSync(path)) {
      throw new EntwineMcpError(
        "binary_not_found",
        `ENTWINE_BIN points to ${path}, which does not exist.`,
      );
    }
    return /\.[cm]?js$/.test(path)
      ? { command: process.execPath, prefix: [path] }
      : { command: path, prefix: [] };
  }
  let manifestPath: string;
  try {
    manifestPath = createRequire(import.meta.url).resolve(
      "@entwine/cli/package.json",
    );
  } catch {
    throw new EntwineMcpError(
      "binary_not_found",
      "The @entwine/cli dependency is not installed. Reinstall @entwine/mcp, or set ENTWINE_BIN to an entwine executable.",
    );
  }
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
    bin?: string | Record<string, string>;
  };
  const bin =
    typeof manifest.bin === "string" ? manifest.bin : manifest.bin?.entwine;
  if (!bin) {
    throw new EntwineMcpError(
      "binary_not_found",
      "@entwine/cli does not declare an entwine launcher.",
    );
  }
  return {
    command: process.execPath,
    prefix: [join(dirname(manifestPath), bin)],
  };
}

/**
 * Run `entwine <subcommand> <root> ...flags` without a shell. The project root is
 * always the canonical server root; callers cannot choose another path.
 */
export function runCli(
  config: Config,
  subcommand: Subcommand,
  flags: readonly string[],
  signal?: AbortSignal,
): Promise<CliResult> {
  return exec(config, subcommand, [subcommand, config.root, ...flags], signal);
}

/** `entwine --version`, e.g. "0.6.0". */
export async function cliVersion(
  config: Config,
  signal?: AbortSignal,
): Promise<string> {
  const result = await exec(config, "--version", ["--version"], signal);
  const match = result.stdout.match(/(\d+\.\d+\.\d+[^\s]*)/);
  if (result.exitCode !== 0 || !match?.[1]) {
    throw new EntwineMcpError(
      "malformed_output",
      `Could not read the Entwine CLI version: ${(result.stderr || result.stdout).trim()}`,
    );
  }
  return match[1];
}

function exec(
  config: Config,
  subcommand: string,
  cliArgs: readonly string[],
  signal?: AbortSignal,
): Promise<CliResult> {
  const { command, prefix } = locateCli(config);
  const args = [...prefix, ...cliArgs];
  return new Promise((resolvePromise, reject) => {
    execFile(
      command,
      args,
      {
        cwd: config.root,
        encoding: "utf8",
        timeout: config.timeoutMs,
        maxBuffer: MAX_BUFFER,
        windowsHide: true,
        shell: false,
        signal,
        env: { ...process.env, NO_COLOR: "1" },
      },
      (error, stdout, stderr) => {
        if (!error) {
          resolvePromise({ exitCode: 0, stdout, stderr });
          return;
        }
        const e = error as NodeJS.ErrnoException & {
          code?: string | number;
          killed?: boolean;
          signal?: string;
        };
        if (e.name === "AbortError" || signal?.aborted) {
          reject(
            new EntwineMcpError("cancelled", "The request was cancelled."),
          );
        } else if (e.killed || e.signal === "SIGTERM") {
          reject(
            new EntwineMcpError(
              "timeout",
              `entwine ${subcommand} exceeded ${config.timeoutMs} ms and was stopped. Raise ENTWINE_MCP_TIMEOUT_MS if the repository is very large.`,
            ),
          );
        } else if (e.code === "ENOENT" || e.code === "EACCES") {
          reject(
            new EntwineMcpError(
              "binary_not_found",
              `Could not execute the Entwine CLI (${command}): ${e.code}. Reinstall @entwine/mcp or set ENTWINE_BIN.`,
            ),
          );
        } else if (typeof e.code === "number") {
          resolvePromise({ exitCode: e.code, stdout, stderr });
        } else {
          reject(
            new EntwineMcpError(
              "cli_failed",
              `entwine ${subcommand} failed to run: ${e.message}`,
            ),
          );
        }
      },
    );
  });
}
