/**
 * Dogfood `entwine init` and `entwine setup` with the real binary, the way a
 * consumer would: scaffold knowledge in a fresh Git repository for every
 * provider, check the generated files, run the commands the pipelines run, and
 * prove a second `setup` changes nothing.
 * Usage: smoke-setup.ts --binary <path-to-entwine>
 */
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({ options: { binary: { type: "string" } } });
if (!values.binary) throw new Error("--binary is required");
const binary = resolve(values.binary);

const providers = [
  {
    name: "GitHub",
    remote: "git@github.com:acme/widgets.git",
    file: ".github/workflows/entwine.yml",
    expect: ["actions/deploy-pages@", "entwine check", "entwine build"],
  },
  {
    name: "GitLab",
    remote: "https://gitlab.com/acme/widgets.git",
    file: ".gitlab/ci/entwine.yml",
    expect: ["$CI_DEFAULT_BRANCH", "publish: dist", "entwine check"],
  },
  {
    name: "Bitbucket",
    remote: "git@bitbucket.org:acme/widgets.git",
    file: "bitbucket-pipelines.yml",
    expect: ["pull-requests:", "ENTWINE_SITE_TOKEN", "entwine build"],
  },
] as const;

const run = (cwd: string, command: string, ...args: string[]): string =>
  execFileSync(command, args, { cwd, encoding: "utf8" });
const entwine = (cwd: string, ...args: string[]): string =>
  run(cwd, binary, ...args);

function snapshot(root: string, dir = root): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.name !== ".git")
    .flatMap((entry) => {
      const path = join(dir, entry.name);
      return entry.isDirectory()
        ? snapshot(root, path)
        : [`${path.slice(root.length)}:${readFileSync(path, "utf8")}`];
    })
    .sort();
}

const work = mkdtempSync(join(tmpdir(), "entwine-setup-"));
try {
  for (const provider of providers) {
    const repo = join(work, provider.name);
    mkdirSync(repo);
    writeFileSync(join(repo, "README.md"), "# Widgets\n");
    run(repo, "git", "init", "-q");
    run(repo, "git", "remote", "add", "origin", provider.remote);

    entwine(repo, "init", "--yes");
    entwine(repo, "check");
    const dry = snapshot(repo);
    const plan = entwine(repo, "setup", "--dry-run");
    if (!plan.includes("No files changed."))
      throw new Error(`${provider.name}: dry run did not say it was a no-op`);
    if (snapshot(repo).join() !== dry.join())
      throw new Error(`${provider.name}: --dry-run changed files`);

    entwine(repo, "setup");
    const generated = readFileSync(join(repo, provider.file), "utf8");
    for (const text of provider.expect)
      if (!generated.includes(text))
        throw new Error(`${provider.name}: ${provider.file} lacks ${text}`);
    if (
      /secrets\.|token=|password/i.test(
        generated.replace("ENTWINE_SITE_TOKEN", ""),
      )
    )
      throw new Error(
        `${provider.name}: generated CI looks like it embeds a credential`,
      );

    const configured = snapshot(repo);
    if (!entwine(repo, "setup").includes("already configured"))
      throw new Error(`${provider.name}: second setup was not idempotent`);
    if (snapshot(repo).join() !== configured.join())
      throw new Error(`${provider.name}: second setup changed files`);

    // The commands every generated pipeline runs.
    entwine(repo, "check");
    entwine(repo, "build");
    if (!existsSync(join(repo, "dist/index.html")))
      throw new Error(`${provider.name}: build produced no dist/index.html`);
    console.log(`✓ ${provider.name}: init, setup, idempotency, check, build`);
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}
