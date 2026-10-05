/** Build exactly what Cloudflare Pages receives: `pnpm build:showcase`. */
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { root } from "./paths.ts";

const exe = process.platform === "win32" ? ".exe" : "";
const entwine = join(root, "target/debug", `entwine${exe}`);
const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";

function run(label: string, command: string, args: string[]): void {
  console.log(`\n▸ ${label}`);
  execFileSync(command, args, {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
}

run("Build Entwine", "cargo", ["build", "--locked", "-p", "entwine-cli"]);
run("Validate Entwine docs", entwine, ["check", "."]);
run("Build Entwine docs → /docs/", entwine, ["build", "."]);
run("Validate kitchen-sink", entwine, ["check", "examples/kitchen-sink"]);
run("Build kitchen-sink → /demo/", entwine, ["build", "examples/kitchen-sink"]);
run("Build Astro website → /", pnpm, ["--filter", "@entwine/www", "build"]);
run("Compose artifact", "node", ["tooling/compose-showcase.ts"]);
run("Verify subpath hosting", "node", ["tooling/verify-showcase.ts"]);
