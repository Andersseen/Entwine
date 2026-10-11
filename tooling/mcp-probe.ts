/**
 * Dependency-free MCP handshake: spawn a stdio server, send `initialize` and
 * `tools/list` as raw JSON-RPC, and return what it answered. Used to verify a
 * published or packed @entwine/mcp without installing an MCP client.
 */
import { spawn } from "node:child_process";

export interface ProbeResult {
  serverName: string;
  tools: string[];
  stray: string[];
}

export function probeMcp(
  command: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv = process.env,
): Promise<ProbeResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let buffer = "";
    let stderr = "";
    const stray: string[] = [];
    const replies = new Map<number, Record<string, unknown>>();
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`MCP probe timed out. stderr: ${stderr}`));
    }, 30_000);
    const send = (message: object): void => {
      child.stdin.write(`${JSON.stringify(message)}\n`);
    };
    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.stdout.on("data", (chunk: Buffer) => {
      buffer += chunk;
      for (let nl = buffer.indexOf("\n"); nl >= 0; nl = buffer.indexOf("\n")) {
        const line = buffer.slice(0, nl).trim();
        buffer = buffer.slice(nl + 1);
        if (!line) continue;
        let message: { jsonrpc?: string; id?: number };
        try {
          message = JSON.parse(line);
        } catch {
          stray.push(line);
          continue;
        }
        if (message.jsonrpc !== "2.0") stray.push(line);
        else if (typeof message.id === "number")
          replies.set(message.id, message as Record<string, unknown>);
        if (replies.has(1) && !replies.has(2)) {
          send({ jsonrpc: "2.0", method: "notifications/initialized" });
          send({ jsonrpc: "2.0", id: 2, method: "tools/list", params: {} });
        }
        if (replies.has(2)) {
          clearTimeout(timer);
          child.stdin.end();
          const init = replies.get(1) as {
            result?: { serverInfo?: { name?: string } };
          };
          const list = replies.get(2) as {
            result?: { tools?: { name: string }[] };
          };
          resolve({
            serverName: init.result?.serverInfo?.name ?? "",
            tools: (list.result?.tools ?? []).map((t) => t.name),
            stray,
          });
        }
      }
    });
    send({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "2025-06-18",
        capabilities: {},
        clientInfo: { name: "entwine-probe", version: "0" },
      },
    });
  });
}
