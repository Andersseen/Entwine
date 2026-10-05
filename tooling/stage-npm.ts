/**
 * Stage npm packages from build outputs; nothing generated is committed.
 *
 *   stage-npm.ts platform --target <triple> --binary <path> --version <v> --out <dir>
 *   stage-npm.ts main --version <v> --out <dir>
 *
 * Platform packages carry one native binary. The main package (`@entwine/cli`)
 * is the compiled launcher plus exact-version optionalDependencies on all of them.
 */
import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { root } from "./paths.ts";
import type { Target } from "./targets.ts";
import { platformPackage, targets } from "./targets.ts";

interface Manifest {
  version: string;
  [key: string]: unknown;
}

const semver = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;
const repository = {
  type: "git",
  url: "git+https://github.com/Andersseen/Entwine.git",
};

function reset(out: string): void {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
}

export function stagePlatform(
  target: Target,
  binary: string,
  version: string,
  out: string,
): string {
  if (!existsSync(binary)) throw new Error(`Binary not found: ${binary}`);
  reset(out);
  const exe = target.os === "win32" ? "entwine.exe" : "entwine";
  mkdirSync(join(out, "bin"));
  copyFileSync(binary, join(out, "bin", exe));
  chmodSync(join(out, "bin", exe), 0o755);
  copyFileSync(join(root, "LICENSE"), join(out, "LICENSE"));
  const name = platformPackage(target);
  const manifest = {
    name,
    version,
    description: `Entwine native binary for ${target.os}-${target.cpu}. Install @entwine/cli instead.`,
    license: "MIT",
    os: [target.os],
    cpu: [target.cpu],
    // Declaring a bin makes npm keep the executable bit; the name avoids clashing with `entwine`.
    bin: { [`entwine-${target.os}-${target.cpu}`]: `bin/${exe}` },
    files: ["bin", "LICENSE"],
    repository,
    publishConfig: { access: "public", provenance: true },
  };
  writeFileSync(
    join(out, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  writeFileSync(
    join(out, "README.md"),
    `# ${name}\n\nNative Entwine binary. Install [\`@entwine/cli\`](https://www.npmjs.com/package/@entwine/cli) instead.\n`,
  );
  return name;
}

export function stageMain(version: string, out: string): void {
  const source = join(root, "packages/cli");
  if (!existsSync(join(source, "dist/launcher.js"))) {
    throw new Error(
      "packages/cli is not built. Run `pnpm --filter @entwine/cli build` first.",
    );
  }
  reset(out);
  cpSync(join(source, "dist"), join(out, "dist"), { recursive: true });
  copyFileSync(join(source, "README.md"), join(out, "README.md"));
  copyFileSync(join(root, "LICENSE"), join(out, "LICENSE"));
  const manifest = JSON.parse(
    readFileSync(join(source, "package.json"), "utf8"),
  ) as Manifest;
  manifest.version = version;
  delete manifest.scripts;
  delete manifest.devDependencies;
  manifest.optionalDependencies = Object.fromEntries(
    targets.map((t) => [platformPackage(t), version]),
  );
  writeFileSync(
    join(out, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
}

function main(): void {
  const [command, ...rest] = process.argv.slice(2);
  const { values } = parseArgs({
    args: rest,
    options: {
      target: { type: "string" },
      binary: { type: "string" },
      version: { type: "string" },
      out: { type: "string" },
    },
  });
  const { version, out } = values;
  if (!version || !semver.test(version))
    throw new Error(`--version must be semver, got ${version}`);
  if (!out) throw new Error("--out is required");
  if (command === "platform") {
    const target = targets.find((t) => t.triple === values.target);
    if (!target || !values.binary)
      throw new Error("--target (a known triple) and --binary are required");
    console.log(
      `✓ staged ${stagePlatform(target, values.binary, version, out)}@${version} in ${out}`,
    );
  } else if (command === "main") {
    stageMain(version, out);
    console.log(`✓ staged @entwine/cli@${version} in ${out}`);
  } else {
    throw new Error("Usage: stage-npm.ts <platform|main> ...");
  }
}

if (process.argv[1]?.endsWith("stage-npm.ts")) main();
