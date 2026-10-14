/** Build exactly what Cloudflare Pages receives: `pnpm build:showcase`. */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./paths.ts";

const exe = process.platform === "win32" ? ".exe" : "";
const entwine = join(root, "target/debug", `entwine${exe}`);
const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const renderer = process.env.ENTWINE_RENDERER;

if (renderer && renderer !== "builtin" && renderer !== "flowview") {
  throw new Error(`Unknown ENTWINE_RENDERER: ${renderer}`);
}

function run(label: string, command: string, args: string[]): void {
  console.log(`\n▸ ${label}`);
  execFileSync(command, args, {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
}

function withRenderer<T>(project: string, action: () => T): T {
  if (!renderer) return action();
  const config = join(project, "entwine.toml");
  const existed = existsSync(config);
  const previous = existed ? readFileSync(config) : undefined;
  let contents = previous?.toString() ?? "";
  if (/^renderer\s*=.*$/m.test(contents)) {
    contents = contents.replace(
      /^renderer\s*=.*$/m,
      `renderer = "${renderer}"`,
    );
  } else if (/^\[site\]\s*$/m.test(contents)) {
    contents = `${contents.trimEnd()}\nrenderer = "${renderer}"\n`;
  } else {
    contents = `${contents.trimEnd()}\n\n[site]\nrenderer = "${renderer}"\n`;
  }
  writeFileSync(config, contents);
  try {
    return action();
  } finally {
    if (previous) writeFileSync(config, previous);
    else unlinkSync(config);
  }
}

run("Build Entwine", "cargo", [
  "build",
  "--locked",
  "-p",
  "entwine-cli",
  ...(renderer === "flowview" ? ["--features", "flowview-renderer"] : []),
]);
run("Validate Entwine docs", entwine, ["check", "."]);
withRenderer(root, () =>
  run("Build Entwine docs → /docs/", entwine, ["build", "."]),
);
run("Validate kitchen-sink", entwine, ["check", "examples/kitchen-sink"]);
withRenderer(join(root, "examples/kitchen-sink"), () =>
  run("Build kitchen-sink → /demo/", entwine, [
    "build",
    "examples/kitchen-sink",
  ]),
);
run("Build Astro website → /", pnpm, ["--filter", "@entwine/www", "build"]);
run("Compose artifact", "node", ["tooling/compose-showcase.ts"]);
run("Verify subpath hosting", "node", ["tooling/verify-showcase.ts"]);
