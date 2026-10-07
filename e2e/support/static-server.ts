/** Minimal static file server for generated Entwine output; no framework, no caching. */

import { readFile, stat } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { extname, join, normalize, sep } from "node:path";

const types: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
  ".json": "application/json",
  ".md": "text/markdown; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
};

export function serve(root: string, port: number): Promise<Server> {
  const server = createServer(async (request, response) => {
    try {
      const pathname = decodeURIComponent(
        new URL(request.url ?? "/", "http://localhost").pathname,
      );
      let file = normalize(join(root, pathname));
      if (file !== root && !file.startsWith(root + sep)) {
        response.writeHead(403).end("Forbidden");
        return;
      }
      if ((await stat(file)).isDirectory()) file = join(file, "index.html");
      const body = await readFile(file);
      response
        .writeHead(200, {
          "content-type": types[extname(file)] ?? "application/octet-stream",
          "cache-control": "no-store",
        })
        .end(body);
    } catch {
      response
        .writeHead(404, { "content-type": "text/plain; charset=utf-8" })
        .end("Not found");
    }
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}
