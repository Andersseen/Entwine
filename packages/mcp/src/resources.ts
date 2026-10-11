import { type McpServer, ResourceTemplate } from "@modelcontextprotocol/server";
import type { Config } from "./config.ts";
import { loadContext, requireDocument } from "./context.ts";
import { buildSummary } from "./tools.ts";
import {
  DOCUMENT_TEMPLATE,
  documentUri,
  idFromVariable,
  SUMMARY_URI,
} from "./uri.ts";

const MAX_LISTED_RESOURCES = 500;

export function registerResources(server: McpServer, config: Config): void {
  server.registerResource(
    "project-summary",
    SUMMARY_URI,
    {
      title: "Entwine project summary",
      description:
        "Counts, roles, entry points and version information for this project's Entwine knowledge.",
      mimeType: "application/json",
    },
    async (uri, ctx) => {
      const context = await loadContext(config, ctx.mcpReq.signal);
      const summary = await buildSummary(config, context, ctx.mcpReq.signal);
      return {
        contents: [
          {
            uri: uri.href,
            mimeType: "application/json",
            text: JSON.stringify(summary, null, 2),
          },
        ],
      };
    },
  );

  server.registerResource(
    "document",
    new ResourceTemplate(DOCUMENT_TEMPLATE, {
      list: async (ctx) => {
        const context = await loadContext(config, ctx.mcpReq.signal);
        return {
          resources: context.documents
            .slice(0, MAX_LISTED_RESOURCES)
            .map((d) => ({
              uri: documentUri(d.id),
              name: d.id,
              title: d.title,
              description: `${d.role} (${d.artifact})`,
              mimeType: "text/markdown",
            })),
        };
      },
    }),
    {
      title: "Entwine document",
      description:
        "A document known to Entwine, addressed by its percent-encoded id (e.g. entwine://document/architecture.md). Returns the exact Markdown source.",
      mimeType: "text/markdown",
    },
    async (uri, variables, ctx) => {
      const context = await loadContext(config, ctx.mcpReq.signal);
      const doc = requireDocument(context, idFromVariable(variables.id ?? ""));
      return {
        contents: [
          { uri: uri.href, mimeType: "text/markdown", text: doc.content },
        ],
      };
    },
  );
}
