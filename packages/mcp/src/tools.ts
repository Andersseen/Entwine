import type { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import type { Config } from "./config.ts";
import {
  type Context,
  type KnowledgeDocument,
  loadContext,
  requireDocument,
} from "./context.ts";
import { EntwineMcpError } from "./errors.ts";
import { cliVersion, runCli } from "./runner.ts";
import { documentUri, SUMMARY_URI } from "./uri.ts";
import { PACKAGE_VERSION, SUPPORTED_SCHEMA_VERSIONS } from "./version.ts";

export const DEFAULT_LIMIT = 50;
export const MAX_LIMIT = 200;
export const DEFAULT_MAX_CHARS = 20_000;
export const MAX_MAX_CHARS = 100_000;
const MAX_OUTPUT_CHARS = 60_000;

const ROLES = [
  "project",
  "architecture",
  "state",
  "roadmap",
  "decision",
  "spec",
  "other",
] as const;
const ARTIFACTS = ["documentation", "agent_instructions", "skill"] as const;

const readOnly = {
  readOnlyHint: true,
  destructiveHint: false,
  idempotentHint: true,
  openWorldHint: false,
} as const;

type ToolResult = {
  content: { type: "text"; text: string }[];
  structuredContent?: Record<string, unknown>;
  isError?: boolean;
};

function ok(text: string, structured: Record<string, unknown>): ToolResult {
  return { content: [{ type: "text", text }], structuredContent: structured };
}

function fail(error: unknown): ToolResult {
  const message =
    error instanceof EntwineMcpError
      ? `${error.code}: ${error.message}`
      : `internal_error: ${error instanceof Error ? error.message : String(error)}`;
  return { content: [{ type: "text", text: message }], isError: true };
}

/** Wrap a handler so failures become actionable tool errors, never crashes. */
function guard<A>(
  handler: (args: A, signal: AbortSignal) => Promise<ToolResult>,
): (args: A, ctx: { mcpReq: { signal: AbortSignal } }) => Promise<ToolResult> {
  return async (args, ctx) => {
    try {
      return await handler(args, ctx.mcpReq.signal);
    } catch (error) {
      return fail(error);
    }
  };
}

function clip(text: string): { text: string; truncated: boolean } {
  return text.length > MAX_OUTPUT_CHARS
    ? { text: text.slice(0, MAX_OUTPUT_CHARS), truncated: true }
    : { text, truncated: false };
}

function summarize(doc: KnowledgeDocument) {
  return {
    id: doc.id,
    title: doc.title,
    role: doc.role,
    artifact: doc.artifact,
    route: doc.route ?? null,
    uri: documentUri(doc.id),
  };
}

function countBy<T>(
  items: T[],
  key: (item: T) => string,
): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const item of items) counts[key(item)] = (counts[key(item)] ?? 0) + 1;
  return Object.fromEntries(Object.entries(counts).sort());
}

/** Shared by the overview tool and the summary resource. */
export async function buildSummary(
  config: Config,
  context: Context,
  signal?: AbortSignal,
) {
  const version = await cliVersion(config, signal);
  return {
    project_root: config.root,
    mcp_version: PACKAGE_VERSION,
    cli_version: version,
    schema_version: context.schemaVersion,
    supported_schema_versions: [...SUPPORTED_SCHEMA_VERSIONS],
    document_count: context.documents.length,
    relationship_count: context.relationships.length,
    by_role: countBy(context.documents, (d) => d.role),
    by_artifact: countBy(context.documents, (d) => d.artifact),
    entry_points: context.documents
      .filter((d) => d.role === "project" && d.artifact === "documentation")
      .map(summarize),
    summary_resource: SUMMARY_URI,
    notes: [
      "Agent instructions and skills (ids starting with repo:) are in the context whenever entwine.toml enables discovery; whether they are published in the static site is controlled separately by [site].include_agent_knowledge.",
    ],
  };
}

const documentSummary = z.object({
  id: z.string(),
  title: z.string(),
  role: z.string(),
  artifact: z.string(),
  route: z.string().nullable(),
  uri: z.string(),
});

export function registerTools(server: McpServer, config: Config): void {
  server.registerTool(
    "entwine_project_overview",
    {
      title: "Entwine project overview",
      description:
        "Summarize the project's Entwine knowledge: counts by role and artifact kind, entry points, CLI/MCP versions and context schema version. Start here.",
      inputSchema: z.object({}),
      outputSchema: z.looseObject({
        project_root: z.string(),
        cli_version: z.string(),
        schema_version: z.string(),
        document_count: z.number(),
      }),
      annotations: readOnly,
    },
    guard(async (_args, signal) => {
      const context = await loadContext(config, signal);
      const summary = await buildSummary(config, context, signal);
      const lines = [
        `Entwine ${summary.cli_version} (context schema ${summary.schema_version}) in ${summary.project_root}`,
        `${summary.document_count} documents, ${summary.relationship_count} relationships`,
        `By role: ${JSON.stringify(summary.by_role)}`,
        `By artifact: ${JSON.stringify(summary.by_artifact)}`,
        `Entry points: ${summary.entry_points.map((d) => d.id).join(", ") || "none"}`,
        summary.notes.join(" "),
      ];
      return ok(lines.join("\n"), summary);
    }),
  );

  server.registerTool(
    "entwine_list_documents",
    {
      title: "List Entwine documents",
      description: `List documents known to Entwine (docs/ plus discovered agent instructions and skills). Bounded: default ${DEFAULT_LIMIT}, max ${MAX_LIMIT} per page; use offset to page. Filter by role, artifact kind, or a case-insensitive query over id and title.`,
      inputSchema: z.object({
        role: z.enum(ROLES).optional(),
        artifact: z.enum(ARTIFACTS).optional(),
        query: z.string().max(200).optional(),
        limit: z.number().int().min(1).max(MAX_LIMIT).default(DEFAULT_LIMIT),
        offset: z.number().int().min(0).default(0),
      }),
      outputSchema: z.object({
        total: z.number(),
        offset: z.number(),
        limit: z.number(),
        has_more: z.boolean(),
        documents: z.array(documentSummary),
      }),
      annotations: readOnly,
    },
    guard(async (args, signal) => {
      const context = await loadContext(config, signal);
      const query = args.query?.toLowerCase();
      const matches = context.documents.filter(
        (d) =>
          (!args.role || d.role === args.role) &&
          (!args.artifact || d.artifact === args.artifact) &&
          (!query ||
            d.id.toLowerCase().includes(query) ||
            d.title.toLowerCase().includes(query)),
      );
      const page = matches
        .slice(args.offset, args.offset + args.limit)
        .map(summarize);
      const result = {
        total: matches.length,
        offset: args.offset,
        limit: args.limit,
        has_more: args.offset + page.length < matches.length,
        documents: page,
      };
      const text = [
        `${page.length} of ${matches.length} documents (offset ${args.offset})${result.has_more ? "; more available" : ""}`,
        ...page.map((d) => `- ${d.id} [${d.role}/${d.artifact}] ${d.title}`),
      ].join("\n");
      return ok(text, result);
    }),
  );

  server.registerTool(
    "entwine_read_document",
    {
      title: "Read an Entwine document",
      description: `Read one document by the id returned from entwine_list_documents. Returns the exact Markdown source (a bounded slice: default ${DEFAULT_MAX_CHARS} chars, max ${MAX_MAX_CHARS}; page with offset), headings, links and backlinks. Only ids known to Entwine are readable; paths are rejected.`,
      inputSchema: z.object({
        id: z.string().min(1).max(1024),
        offset: z.number().int().min(0).default(0),
        max_chars: z
          .number()
          .int()
          .min(1)
          .max(MAX_MAX_CHARS)
          .default(DEFAULT_MAX_CHARS),
      }),
      outputSchema: z.object({
        id: z.string(),
        title: z.string(),
        role: z.string(),
        artifact: z.string(),
        route: z.string().nullable(),
        uri: z.string(),
        metadata: z.looseObject({}),
        headings: z.array(
          z.object({ level: z.number(), text: z.string(), id: z.string() }),
        ),
        links: z.array(z.string()),
        backlinks: z.array(z.string()),
        content: z.string(),
        offset: z.number(),
        total_chars: z.number(),
        truncated: z.boolean(),
      }),
      annotations: readOnly,
    },
    guard(async (args, signal) => {
      const context = await loadContext(config, signal);
      const doc = requireDocument(context, args.id);
      const content = doc.content.slice(
        args.offset,
        args.offset + args.max_chars,
      );
      const truncated = args.offset + content.length < doc.content.length;
      const result = {
        ...summarize(doc),
        metadata: doc.metadata,
        headings: doc.headings,
        links: [...new Set(doc.links.map((l) => l.target ?? l.destination))],
        backlinks: doc.backlinks,
        content,
        offset: args.offset,
        total_chars: doc.content.length,
        truncated,
      };
      const note = truncated
        ? `\n\n[truncated at ${args.offset + content.length} of ${doc.content.length} chars; call again with offset=${args.offset + content.length}]`
        : "";
      return ok(`# ${doc.id}\n\n${content}${note}`, result);
    }),
  );

  server.registerTool(
    "entwine_related_documents",
    {
      title: "Related Entwine documents",
      description:
        "List documents related to one document through Entwine's knowledge relationships (outgoing references, incoming backlinks, or both), with each neighbour's role.",
      inputSchema: z.object({
        id: z.string().min(1).max(1024),
        direction: z.enum(["outgoing", "incoming", "both"]).default("both"),
      }),
      outputSchema: z.object({
        id: z.string(),
        outgoing: z.array(documentSummary.extend({ kind: z.string() })),
        incoming: z.array(documentSummary.extend({ kind: z.string() })),
      }),
      annotations: readOnly,
    },
    guard(async (args, signal) => {
      const context = await loadContext(config, signal);
      const doc = requireDocument(context, args.id);
      const neighbours = (edges: { id: string; kind: string }[]) =>
        edges.flatMap((edge) => {
          const found = context.byId.get(edge.id);
          return found ? [{ ...summarize(found), kind: edge.kind }] : [];
        });
      const outgoing =
        args.direction === "incoming"
          ? []
          : neighbours(
              context.relationships
                .filter((r) => r.source === doc.id)
                .map((r) => ({ id: r.target, kind: r.kind })),
            );
      const incoming =
        args.direction === "outgoing"
          ? []
          : neighbours(
              context.relationships
                .filter((r) => r.target === doc.id)
                .map((r) => ({ id: r.source, kind: r.kind })),
            );
      const text = [
        `${doc.id}: ${outgoing.length} outgoing, ${incoming.length} incoming`,
        ...outgoing.map((d) => `  -> ${d.id} [${d.role}] (${d.kind})`),
        ...incoming.map((d) => `  <- ${d.id} [${d.role}] (${d.kind})`),
      ].join("\n");
      return ok(text, { id: doc.id, outgoing, incoming });
    }),
  );

  server.registerTool(
    "entwine_check",
    {
      title: "Validate project documentation",
      description:
        "Run `entwine check` (read-only): validates links and anchors and reports knowledge coverage. A failing check is reported as passed=false with the CLI output, not as a tool error. Set strict_knowledge to fail when a recommended area is missing.",
      inputSchema: z.object({
        strict_knowledge: z.boolean().default(false),
      }),
      outputSchema: z.object({
        passed: z.boolean(),
        exit_code: z.number(),
        strict_knowledge: z.boolean(),
        output: z.string(),
        truncated: z.boolean(),
      }),
      annotations: readOnly,
    },
    guard(async (args, signal) => {
      const flags = args.strict_knowledge ? ["--strict-knowledge"] : [];
      const result = await runCli(config, "check", flags, signal);
      const { text, truncated } = clip(
        [result.stdout, result.stderr].filter(Boolean).join("\n").trim(),
      );
      const passed = result.exitCode === 0;
      return ok(
        `${passed ? "PASSED" : "FAILED"} (exit ${result.exitCode})\n${text}`,
        {
          passed,
          exit_code: result.exitCode,
          strict_knowledge: args.strict_knowledge,
          output: text,
          truncated,
        },
      );
    }),
  );

  const previewOutput = z.object({
    dry_run: z.literal(true),
    changed_files: z.literal(false),
    exit_code: z.number(),
    output: z.string(),
    truncated: z.boolean(),
  });

  async function preview(
    subcommand: "init" | "setup",
    flags: string[],
    signal: AbortSignal,
  ): Promise<ToolResult> {
    // `--dry-run` is hard-coded: these tools can never write.
    const result = await runCli(
      config,
      subcommand,
      [...flags, "--dry-run"],
      signal,
    );
    const { text, truncated } = clip(
      [result.stdout, result.stderr].filter(Boolean).join("\n").trim(),
    );
    if (result.exitCode !== 0) {
      throw new EntwineMcpError(
        "cli_failed",
        `entwine ${subcommand} --dry-run failed (exit ${result.exitCode}): ${text}`,
      );
    }
    return ok(text, {
      dry_run: true,
      changed_files: false,
      exit_code: result.exitCode,
      output: text,
      truncated,
    });
  }

  server.registerTool(
    "entwine_init_preview",
    {
      title: "Preview entwine init",
      description:
        "Run `entwine init --dry-run`: shows which knowledge files would be scaffolded. Never writes. To apply, the user runs `entwine init` themselves.",
      inputSchema: z.object({}),
      outputSchema: previewOutput,
      annotations: readOnly,
    },
    guard((_args, signal) => preview("init", [], signal)),
  );

  server.registerTool(
    "entwine_setup_preview",
    {
      title: "Preview entwine setup",
      description:
        "Run `entwine setup --dry-run`: shows the CI/publishing files that would be generated for the detected (or given) Git provider. Never writes and never deploys. To apply, the user runs `entwine setup` themselves.",
      inputSchema: z.object({
        provider: z.enum(["github", "gitlab", "bitbucket"]).optional(),
      }),
      outputSchema: previewOutput,
      annotations: readOnly,
    },
    guard((args, signal) =>
      preview(
        "setup",
        args.provider ? ["--provider", args.provider] : [],
        signal,
      ),
    ),
  );
}
