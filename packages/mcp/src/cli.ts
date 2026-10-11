#!/usr/bin/env node
// stdout carries MCP protocol messages only; everything else goes to stderr.
import { serveStdio } from "@modelcontextprotocol/server/stdio";
import { loadConfig } from "./config.ts";
import { EntwineMcpError } from "./errors.ts";
import { createServer } from "./server.ts";
import { PACKAGE_VERSION, SUPPORTED_SCHEMA_VERSIONS } from "./version.ts";

const HELP = `entwine-mcp ${PACKAGE_VERSION} — Entwine MCP server (stdio)

Usage: entwine-mcp [--root <dir>]

  --root <dir>   Project root (default: $ENTWINE_PROJECT_ROOT, else the working directory)
  --version      Print the version and supported context schema versions
  --help         Print this help

Environment:
  ENTWINE_PROJECT_ROOT   Project root
  ENTWINE_BIN            Use this entwine executable instead of the bundled @entwine/cli
  ENTWINE_MCP_TIMEOUT_MS Per-call CLI timeout (default 60000)
`;

const argv = process.argv.slice(2);
if (argv.includes("--help") || argv.includes("-h")) {
  process.stderr.write(HELP);
  process.exit(0);
}
if (argv.includes("--version") || argv.includes("-V")) {
  process.stdout.write(
    `${PACKAGE_VERSION} (context schema ${SUPPORTED_SCHEMA_VERSIONS.join(", ")})\n`,
  );
  process.exit(0);
}

try {
  const config = loadConfig(argv, process.env, process.cwd());
  process.stderr.write(
    `entwine-mcp ${PACKAGE_VERSION}: serving ${config.root}\n`,
  );
  serveStdio(() => createServer(config), {
    onerror: (error) => process.stderr.write(`entwine-mcp: ${error.message}\n`),
  });
} catch (error) {
  const message =
    error instanceof EntwineMcpError || error instanceof Error
      ? error.message
      : String(error);
  process.stderr.write(`entwine-mcp: ${message}\n`);
  process.exit(1);
}
