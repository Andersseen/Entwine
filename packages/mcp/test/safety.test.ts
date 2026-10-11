import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import {
  bareRepo,
  cleanup,
  connect,
  entwineBinary,
  exists,
  fakeCli,
  gitStatus,
  serverEntry,
  structured,
  tempDir,
  text,
} from "./helpers.ts";

type Session = Awaited<ReturnType<typeof connect>>;
const call = (c: Session, name: string, args = {}) =>
  c.client.callTool({ name, arguments: args });

function docsRepo(index = "# Home\n"): string {
  const repo = tempDir();
  mkdirSync(join(repo, "docs"));
  writeFileSync(join(repo, "docs/index.md"), index);
  return repo;
}

describe("empty and missing documentation", () => {
  it("reports an actionable error without docs/, while previews still work", async () => {
    const repo = bareRepo();
    const s = await connect(repo);
    try {
      const overview = await call(s, "entwine_project_overview");
      assert.equal(overview.isError, true);
      assert.match(text(overview), /docs/);
      const check = structured<{ passed: boolean }>(
        await call(s, "entwine_check"),
      );
      assert.equal(check.passed, false);
      const init = await call(s, "entwine_init_preview");
      assert.match(text(init), /Would create/);
      assert.match(text(init), /docs\/index\.md/);
      const setup = await call(s, "entwine_setup_preview");
      assert.match(text(setup), /Provider: GitHub/);
      assert.equal(exists(join(repo, "docs")), false);
      assert.equal(exists(join(repo, ".github")), false);
      assert.equal(exists(join(repo, "dist")), false);
      assert.equal(gitStatus(repo), "");
    } finally {
      await s.close();
      cleanup(repo);
    }
  });

  it("surfaces a failing check as passed=false with CLI output", async () => {
    const repo = docsRepo("# Home\n\n[gone](missing.md)\n");
    const s = await connect(repo);
    try {
      const check = structured<{ passed: boolean; output: string }>(
        await call(s, "entwine_check"),
      );
      assert.equal(check.passed, false);
      assert.match(check.output, /Broken internal link/);
      assert.equal(exists(join(repo, "dist")), false);
    } finally {
      await s.close();
      cleanup(repo);
    }
  });
});

describe("path and injection boundaries", () => {
  it("rejects traversal, absolute paths, unknown ids and injection strings", async () => {
    const repo = docsRepo();
    const outside = tempDir();
    writeFileSync(join(outside, "secret.md"), "TOP-SECRET");
    const s = await connect(repo);
    try {
      for (const id of [
        "../secret.md",
        `../${outside}/secret.md`,
        join(outside, "secret.md"),
        "docs/index.md",
        "index.md; touch pwned",
        "$(touch pwned)",
        "repo:../../etc/passwd",
      ]) {
        const result = await call(s, "entwine_read_document", { id });
        assert.equal(result.isError, true, id);
        assert.match(text(result), /unknown_document/, id);
        assert.doesNotMatch(text(result), /TOP-SECRET/);
      }
      await assert.rejects(
        s.client.readResource({
          uri: `entwine://document/${encodeURIComponent(join(outside, "secret.md"))}`,
        }),
      );
      const bad = await call(s, "entwine_setup_preview", {
        provider: "github; touch pwned",
      });
      assert.equal(bad.isError, true);
      const none = await call(s, "entwine_list_documents", {
        query: "$(touch pwned) `id` ; &&",
      });
      assert.equal(structured<{ total: number }>(none).total, 0);
      assert.equal(exists(join(repo, "pwned")), false);
    } finally {
      await s.close();
      cleanup(repo, outside);
    }
  });

  it("does not read through symlinks that escape the project", async () => {
    const repo = docsRepo();
    const outside = tempDir();
    writeFileSync(join(outside, "secret.md"), "TOP-SECRET");
    try {
      symlinkSync(join(outside, "secret.md"), join(repo, "docs/leak.md"));
    } catch {
      cleanup(repo, outside);
      return; // symlinks unavailable (unprivileged Windows)
    }
    const s = await connect(repo);
    try {
      const listed = await call(s, "entwine_list_documents");
      const read = await call(s, "entwine_read_document", { id: "leak.md" });
      assert.doesNotMatch(text(listed) + text(read), /TOP-SECRET/);
      assert.equal(read.isError, true);
    } finally {
      await s.close();
      cleanup(repo, outside);
    }
  });
});

describe("root and configuration", () => {
  it("fails to start on an invalid root, writing only to stderr", async () => {
    const child = spawn(
      process.execPath,
      [serverEntry, "--root", join(tempDir(), "missing")],
      {
        stdio: ["ignore", "pipe", "pipe"],
        env: { ...process.env, ENTWINE_BIN: entwineBinary },
      },
    );
    let out = "";
    let err = "";
    child.stdout.on("data", (d: Buffer) => {
      out += d;
    });
    child.stderr.on("data", (d: Buffer) => {
      err += d;
    });
    const code = await new Promise((r) => child.on("close", r));
    assert.equal(code, 1);
    assert.equal(out, "");
    assert.match(err, /does not exist/);
  });

  it("honours ENTWINE_PROJECT_ROOT", async () => {
    const repo = docsRepo("# Env root\n");
    const viaEnv = await connect(tempDir(), { ENTWINE_PROJECT_ROOT: repo }, []);
    try {
      const overview = structured<{ project_root: string }>(
        await call(viaEnv, "entwine_project_overview"),
      );
      assert.equal(overview.project_root, repo);
    } finally {
      await viaEnv.close();
      cleanup(repo);
    }
  });

  it("keeps stdout limited to JSON-RPC protocol messages", async () => {
    const repo = docsRepo();
    const child = spawn(process.execPath, [serverEntry, "--root", repo], {
      stdio: ["pipe", "pipe", "pipe"],
      env: { ...process.env, ENTWINE_BIN: entwineBinary },
    });
    let out = "";
    child.stdout.on("data", (d: Buffer) => {
      out += d;
    });
    child.stdin.write(
      `${JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2025-06-18",
          capabilities: {},
          clientInfo: { name: "raw", version: "0" },
        },
      })}\n`,
    );
    await new Promise((r) => setTimeout(r, 1500));
    child.stdin.end();
    await new Promise((r) => child.on("close", r));
    const lines = out.split("\n").filter(Boolean);
    assert.ok(lines.length >= 1);
    for (const line of lines) {
      assert.equal((JSON.parse(line) as { jsonrpc: string }).jsonrpc, "2.0");
    }
    cleanup(repo);
  });
});

describe("CLI faults are reported, not fatal", () => {
  async function withFake(body: string, env: Record<string, string> = {}) {
    const repo = docsRepo();
    const bin = fakeCli(repo, body);
    const s = await connect(repo, { ENTWINE_BIN: bin, ...env });
    return { repo, s };
  }
  const finish = async (x: { repo: string; s: Session }) => {
    await x.s.close();
    cleanup(x.repo);
  };

  it("rejects unsupported context schema versions", async () => {
    const x = await withFake(
      `console.log(JSON.stringify({schema_version:"9.9",documents:[],relationships:[]}))`,
    );
    try {
      const r = await call(x.s, "entwine_list_documents");
      assert.equal(r.isError, true);
      assert.match(text(r), /unsupported_schema/);
      assert.match(text(r), /9\.9/);
    } finally {
      await finish(x);
    }
  });

  it("rejects malformed JSON and malformed documents", async () => {
    const bad = await withFake(`console.log("not json")`);
    try {
      assert.match(
        text(await call(bad.s, "entwine_list_documents")),
        /malformed_output/,
      );
    } finally {
      await finish(bad);
    }
    const shape = await withFake(
      `console.log(JSON.stringify({schema_version:"0.4",documents:[{id:1}],relationships:[]}))`,
    );
    try {
      assert.match(
        text(await call(shape.s, "entwine_list_documents")),
        /malformed_output/,
      );
    } finally {
      await finish(shape);
    }
  });

  it("reports nonzero exits, missing binaries and timeouts", async () => {
    const fail = await withFake(`console.error("boom"); process.exit(3)`);
    try {
      assert.match(
        text(await call(fail.s, "entwine_list_documents")),
        /cli_failed[\s\S]*boom/,
      );
    } finally {
      await finish(fail);
    }

    const missing = await connect(tempDir(), {
      ENTWINE_BIN: join(tempDir(), "no-such-entwine"),
    });
    try {
      assert.match(
        text(await call(missing, "entwine_check")),
        /binary_not_found/,
      );
    } finally {
      await missing.close();
    }

    const slow = await withFake(`setTimeout(() => {}, 60000)`, {
      ENTWINE_MCP_TIMEOUT_MS: "700",
    });
    try {
      const started = Date.now();
      const r = await call(slow.s, "entwine_list_documents");
      assert.match(text(r), /timeout/);
      assert.ok(Date.now() - started < 10_000);
      assert.equal((await slow.s.client.listTools()).tools.length, 7);
    } finally {
      await finish(slow);
    }
  });

  it("bounds oversized document output", async () => {
    const big = "x".repeat(500_000);
    const x = await withFake(
      `console.log(JSON.stringify({schema_version:"0.4",documents:[{id:"big.md",title:"Big",role:"other",artifact:"documentation",content:${JSON.stringify(big)}}],relationships:[]}))`,
    );
    try {
      const r = await call(x.s, "entwine_read_document", { id: "big.md" });
      assert.ok(text(r).length < 25_000);
      const data = structured<{ truncated: boolean; total_chars: number }>(r);
      assert.equal(data.truncated, true);
      assert.equal(data.total_chars, 500_000);
      const over = await call(x.s, "entwine_read_document", {
        id: "big.md",
        max_chars: 10_000_000,
      });
      assert.equal(over.isError, true);
    } finally {
      await finish(x);
    }
  });
});
