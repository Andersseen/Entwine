import * as z from "zod/v4";
import type { Config } from "./config.ts";
import { EntwineMcpError } from "./errors.ts";
import { runCli } from "./runner.ts";
import { SUPPORTED_SCHEMA_VERSIONS } from "./version.ts";

const link = z.object({
  destination: z.string(),
  line: z.number().optional(),
  target: z.string().nullable().optional(),
  href: z.string().nullable().optional(),
});

const document = z.looseObject({
  id: z.string(),
  title: z.string(),
  route: z.string().nullable().optional(),
  metadata: z
    .looseObject({
      title: z.string().nullable().optional(),
      type: z.string().nullable().optional(),
      status: z.string().nullable().optional(),
    })
    .default({}),
  role: z.string(),
  headings: z
    .array(z.object({ level: z.number(), text: z.string(), id: z.string() }))
    .default([]),
  links: z.array(link).default([]),
  backlinks: z.array(z.string()).default([]),
  anchors: z.array(z.unknown()).default([]),
  repository_references: z.array(z.unknown()).default([]),
  artifact: z.string(),
  agent: z.unknown().optional(),
  content: z.string(),
});

const relationship = z.object({
  source: z.string(),
  target: z.string(),
  kind: z.string(),
});

const contextSchema = z.object({
  schema_version: z.string(),
  documents: z.array(document),
  relationships: z.array(relationship),
});

export type KnowledgeDocument = z.infer<typeof document>;
export type Relationship = z.infer<typeof relationship>;

export interface Context {
  schemaVersion: string;
  documents: KnowledgeDocument[];
  relationships: Relationship[];
  byId: Map<string, KnowledgeDocument>;
}

/** Parse and validate `entwine context --json`. Never guesses at unknown schemas. */
export function parseContext(stdout: string): Context {
  let raw: unknown;
  try {
    raw = JSON.parse(stdout);
  } catch {
    throw new EntwineMcpError(
      "malformed_output",
      "`entwine context --json` did not produce valid JSON. The installed CLI may be incompatible with this @entwine/mcp version.",
    );
  }
  const version =
    typeof raw === "object" && raw !== null
      ? (raw as { schema_version?: unknown }).schema_version
      : undefined;
  if (typeof version !== "string") {
    throw new EntwineMcpError(
      "malformed_output",
      "The Entwine context has no schema_version. Upgrade @entwine/cli and @entwine/mcp together.",
    );
  }
  if (!SUPPORTED_SCHEMA_VERSIONS.includes(version)) {
    throw new EntwineMcpError(
      "unsupported_schema",
      `Entwine context schema_version ${JSON.stringify(version)} is not supported by this @entwine/mcp (supports ${SUPPORTED_SCHEMA_VERSIONS.join(", ")}). Install matching versions of @entwine/cli and @entwine/mcp.`,
    );
  }
  const parsed = contextSchema.safeParse(raw);
  if (!parsed.success) {
    throw new EntwineMcpError(
      "malformed_output",
      `Entwine context does not match schema ${version}: ${z.prettifyError(parsed.error)}`,
    );
  }
  const { documents, relationships } = parsed.data;
  return {
    schemaVersion: version,
    documents,
    relationships,
    byId: new Map(documents.map((d) => [d.id, d])),
  };
}

/** Load a fresh context from the CLI; files may change between calls. */
export async function loadContext(
  config: Config,
  signal?: AbortSignal,
): Promise<Context> {
  const result = await runCli(config, "context", ["--json"], signal);
  if (result.exitCode !== 0) {
    const detail = (result.stderr || result.stdout).trim();
    if (/no prebuilt binary/i.test(detail)) {
      throw new EntwineMcpError("unsupported_platform", detail);
    }
    throw new EntwineMcpError(
      "cli_failed",
      `entwine context failed (exit ${result.exitCode}): ${detail || "no output"}`,
    );
  }
  return parseContext(result.stdout);
}

export function requireDocument(
  context: Context,
  id: string,
): KnowledgeDocument {
  const found = context.byId.get(id);
  if (!found) {
    throw new EntwineMcpError(
      "unknown_document",
      `No document with id ${JSON.stringify(id)}. Ids come from entwine_list_documents (for example "architecture.md" or "repo:AGENTS.md").`,
    );
  }
  return found;
}
