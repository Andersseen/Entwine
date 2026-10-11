import { realpathSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { EntwineMcpError } from "./errors.ts";

export interface Config {
  /** Canonical absolute project root; every CLI call is scoped to it. */
  root: string;
  /** Explicit CLI override (ENTWINE_BIN); otherwise the bundled @entwine/cli. */
  binary: string | undefined;
  timeoutMs: number;
}

export const DEFAULT_TIMEOUT_MS = 60_000;

/** Canonicalize and validate a project root. Throws an actionable error. */
export function resolveRoot(input: string): string {
  const absolute = resolve(input);
  let real: string;
  try {
    real = realpathSync(absolute);
  } catch {
    throw new EntwineMcpError(
      "invalid_root",
      `Project root ${absolute} does not exist. Pass --root <dir> or set ENTWINE_PROJECT_ROOT to an existing repository directory.`,
    );
  }
  if (!statSync(real).isDirectory()) {
    throw new EntwineMcpError(
      "invalid_root",
      `Project root ${real} is not a directory.`,
    );
  }
  return real;
}

export function loadConfig(
  argv: readonly string[],
  env: NodeJS.ProcessEnv,
  cwd: string,
): Config {
  let rootArg: string | undefined;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--root") {
      rootArg = argv[++i];
      if (!rootArg)
        throw new EntwineMcpError("invalid_root", "--root needs a directory.");
    } else if (arg?.startsWith("--root=")) {
      rootArg = arg.slice("--root=".length);
    } else {
      throw new EntwineMcpError(
        "invalid_input",
        `Unknown argument ${JSON.stringify(arg)}. Supported: --root <dir>, --version, --help.`,
      );
    }
  }
  const timeout = Number(env.ENTWINE_MCP_TIMEOUT_MS ?? DEFAULT_TIMEOUT_MS);
  return {
    root: resolveRoot(rootArg ?? env.ENTWINE_PROJECT_ROOT ?? cwd),
    binary: env.ENTWINE_BIN || undefined,
    timeoutMs:
      Number.isFinite(timeout) && timeout > 0 ? timeout : DEFAULT_TIMEOUT_MS,
  };
}
