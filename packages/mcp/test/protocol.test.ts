import assert from "node:assert/strict";
import { after, before, describe, it } from "node:test";
import {
  type Connected,
  cleanup,
  cli,
  connect,
  gitStatus,
  promptText,
  representativeRepo,
  resourceText,
  structured,
  text,
} from "./helpers.ts";

interface Doc {
  id: string;
  title: string;
  role: string;
  artifact: string;
  content: string;
  backlinks: string[];
}
interface Ctx {
  schema_version: string;
  documents: Doc[];
  relationships: { source: string; target: string; kind: string }[];
}

describe("MCP protocol against a real stdio server", () => {
  let repo: string;
  let session: Connected;
  let truth: Ctx;

  before(async () => {
    repo = representativeRepo();
    truth = JSON.parse(cli(["context", repo, "--json"])) as Ctx;
    session = await connect(repo);
  });
  after(async () => {
    await session.close();
    cleanup(repo);
  });

  it("initializes with tools, resources and prompts capabilities", () => {
    const caps = session.client.getServerCapabilities();
    assert.ok(caps?.tools && caps.resources && caps.prompts);
    assert.equal(session.client.getServerVersion()?.name, "entwine");
    assert.match(
      session.client.getInstructions() ?? "",
      /entwine_project_overview/,
    );
  });

  it("lists the focused tool set with read-only annotations", async () => {
    const { tools } = await session.client.listTools();
    assert.deepEqual(tools.map((t) => t.name).sort(), [
      "entwine_check",
      "entwine_init_preview",
      "entwine_list_documents",
      "entwine_project_overview",
      "entwine_read_document",
      "entwine_related_documents",
      "entwine_setup_preview",
    ]);
    for (const tool of tools) {
      assert.equal(tool.annotations?.readOnlyHint, true, tool.name);
      assert.equal(tool.annotations?.destructiveHint, false, tool.name);
      assert.ok(tool.inputSchema && tool.outputSchema, tool.name);
    }
  });

  it("lists resources, a document template and prompts", async () => {
    const { resources } = await session.client.listResources();
    assert.ok(resources.some((r) => r.uri === "entwine://project/summary"));
    assert.equal(
      resources.filter((r) => r.uri.startsWith("entwine://document/")).length,
      truth.documents.length,
    );
    const { resourceTemplates } = await session.client.listResourceTemplates();
    assert.equal(resourceTemplates[0]?.uriTemplate, "entwine://document/{id}");
    const { prompts } = await session.client.listPrompts();
    assert.deepEqual(prompts.map((p) => p.name).sort(), [
      "entwine_adopt",
      "entwine_maintain_knowledge",
      "entwine_prepare_publishing",
    ]);
  });

  it("matches the CLI context for ids, roles and artifacts", async () => {
    const all = [];
    for (let offset = 0; ; offset += 200) {
      const page = structured<{
        documents: Doc[];
        has_more: boolean;
        total: number;
      }>(
        await session.client.callTool({
          name: "entwine_list_documents",
          arguments: { limit: 200, offset },
        }),
      );
      all.push(...page.documents);
      assert.equal(page.total, truth.documents.length);
      if (!page.has_more) break;
    }
    assert.deepEqual(
      all.map((d) => [d.id, d.role, d.artifact]),
      truth.documents.map((d) => [d.id, d.role, d.artifact]),
    );
    assert.ok(all.some((d) => d.id === "guides/deep/nested.md"));
    assert.ok(all.some((d) => d.artifact === "skill"));
    assert.ok(all.some((d) => d.artifact === "agent_instructions"));
  });

  it("filters and bounds listings", async () => {
    const decisions = structured<{ documents: Doc[]; total: number }>(
      await session.client.callTool({
        name: "entwine_list_documents",
        arguments: { role: "decision" },
      }),
    );
    assert.equal(
      decisions.total,
      truth.documents.filter((d) => d.role === "decision").length,
    );
    const one = structured<{ documents: Doc[]; has_more: boolean }>(
      await session.client.callTool({
        name: "entwine_list_documents",
        arguments: { limit: 1 },
      }),
    );
    assert.equal(one.documents.length, 1);
    assert.equal(one.has_more, true);
    const tooBig = await session.client.callTool({
      name: "entwine_list_documents",
      arguments: { limit: 5000 },
    });
    assert.equal(tooBig.isError, true);
  });

  it("returns exact source content, and pages long documents", async () => {
    for (const doc of truth.documents) {
      const read = structured<{ content: string; truncated: boolean }>(
        await session.client.callTool({
          name: "entwine_read_document",
          arguments: { id: doc.id },
        }),
      );
      assert.equal(read.content, doc.content, doc.id);
    }
    const first = truth.documents[0];
    assert.ok(first);
    const a = structured<{ content: string; truncated: boolean }>(
      await session.client.callTool({
        name: "entwine_read_document",
        arguments: { id: first.id, max_chars: 10 },
      }),
    );
    assert.equal(a.content, first.content.slice(0, 10));
    assert.equal(a.truncated, true);
    const b = structured<{ content: string }>(
      await session.client.callTool({
        name: "entwine_read_document",
        arguments: { id: first.id, offset: 10, max_chars: 10 },
      }),
    );
    assert.equal(b.content, first.content.slice(10, 20));
  });

  it("returns relations that match the CLI relationships", async () => {
    const id = "architecture.md";
    const related = structured<{
      outgoing: { id: string }[];
      incoming: { id: string }[];
    }>(
      await session.client.callTool({
        name: "entwine_related_documents",
        arguments: { id },
      }),
    );
    assert.deepEqual(
      related.outgoing.map((d) => d.id).sort(),
      truth.relationships
        .filter((r) => r.source === id)
        .map((r) => r.target)
        .sort(),
    );
    assert.deepEqual(
      related.incoming.map((d) => d.id).sort(),
      truth.relationships
        .filter((r) => r.target === id)
        .map((r) => r.source)
        .sort(),
    );
    const arch = truth.documents.find((d) => d.id === id);
    assert.deepEqual(
      related.incoming.map((d) => d.id).sort(),
      [...(arch?.backlinks ?? [])].sort(),
    );
  });

  it("reads the summary and documents as resources, including encoded ids", async () => {
    const summary = await session.client.readResource({
      uri: "entwine://project/summary",
    });
    const parsed = JSON.parse(resourceText(summary)) as {
      schema_version: string;
      document_count: number;
    };
    assert.equal(parsed.schema_version, truth.schema_version);
    assert.equal(parsed.document_count, truth.documents.length);

    for (const id of ["architecture.md", "my notes.md", "repo:AGENTS.md"]) {
      const doc = truth.documents.find((d) => d.id === id);
      if (!doc) continue;
      const res = await session.client.readResource({
        uri: `entwine://document/${encodeURIComponent(id)}`,
      });
      assert.equal(resourceText(res), doc.content, id);
    }
    assert.ok(truth.documents.some((d) => d.id === "my notes.md"));
    await assert.rejects(
      session.client.readResource({ uri: "entwine://document/nope.md" }),
    );
  });

  it("overview reports versions and counts", async () => {
    const overview = structured<{
      cli_version: string;
      schema_version: string;
      by_artifact: Record<string, number>;
    }>(
      await session.client.callTool({
        name: "entwine_project_overview",
        arguments: {},
      }),
    );
    assert.equal(overview.schema_version, truth.schema_version);
    assert.match(overview.cli_version, /^\d+\.\d+\.\d+/);
    assert.ok((overview.by_artifact.skill ?? 0) > 0);
  });

  it("serves prompts that reference real tools", async () => {
    for (const name of [
      "entwine_adopt",
      "entwine_maintain_knowledge",
      "entwine_prepare_publishing",
    ]) {
      const prompt = await session.client.getPrompt({ name });
      const body = promptText(prompt);
      assert.match(
        body,
        /entwine_(check|init_preview|setup_preview|list_documents|project_overview)/,
      );
    }
  });

  it("reports check results and previews without changing the repository", async () => {
    const before = gitStatus(repo);
    const check = structured<{ passed: boolean; output: string }>(
      await session.client.callTool({ name: "entwine_check", arguments: {} }),
    );
    assert.equal(check.passed, true);
    assert.match(check.output, /validation passed/);

    const init = await session.client.callTool({
      name: "entwine_init_preview",
      arguments: {},
    });
    assert.match(text(init), /already configured|No files changed/);
    const setup = structured<{ output: string; changed_files: boolean }>(
      await session.client.callTool({
        name: "entwine_setup_preview",
        arguments: { provider: "github" },
      }),
    );
    assert.match(setup.output, /No files changed/);
    assert.equal(setup.changed_files, false);
    assert.equal(gitStatus(repo), before);
    assert.equal(gitStatus(repo).includes("dist"), false);
  });
});
