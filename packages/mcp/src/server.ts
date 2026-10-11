import { McpServer } from "@modelcontextprotocol/server";
import type { Config } from "./config.ts";
import { registerPrompts } from "./prompts.ts";
import { registerResources } from "./resources.ts";
import { registerTools } from "./tools.ts";
import { PACKAGE_VERSION } from "./version.ts";

const INSTRUCTIONS =
  "Entwine turns a repository's Markdown into a knowledge model. Start with entwine_project_overview, list documents with entwine_list_documents, read with entwine_read_document or the entwine://document/{id} resource, and validate with entwine_check. entwine_init_preview and entwine_setup_preview are dry runs; this server never writes files, deploys or publishes.";

export function createServer(config: Config): McpServer {
  const server = new McpServer(
    { name: "entwine", version: PACKAGE_VERSION },
    {
      capabilities: { tools: {}, resources: {}, prompts: {} },
      instructions: INSTRUCTIONS,
    },
  );
  registerTools(server, config);
  registerResources(server, config);
  registerPrompts(server);
  return server;
}
