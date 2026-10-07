/**
 * Pure release-lifecycle decisions plus a thin CLI used by the Release workflow.
 *
 * Invariant: while a generated release is an unresolved draft, release-please
 * must not run, because the draft has no Git tag yet and release-please would
 * count every already-shipped commit as unreleased and open another release PR.
 *
 *   release-state.ts guard   --releases <json>            (push to main)
 *   release-state.ts plan    --tag vX.Y.Z --assets <json> (resume a draft)
 */
import { appendFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { lookupVersion } from "./registry.ts";
import { platformPackage, targets } from "./targets.ts";

export interface ReleaseSummary {
  tagName: string;
  isDraft: boolean;
}

export type PendingRelease =
  | { status: "none" }
  | { status: "pending"; tag: string }
  | { status: "multiple"; tags: string[] };

const releaseTag = /^v\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

const compareTags = (a: string, b: string): number => {
  const parts = (tag: string) => tag.slice(1).split(/[.-]/).map(Number);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < 3; i++)
    if (x[i] !== y[i]) return (x[i] ?? 0) - (y[i] ?? 0);
  return 0;
};

/** Which Entwine release drafts are still waiting to be published? */
export function assessPendingReleases(
  releases: readonly ReleaseSummary[],
): PendingRelease {
  const tags = releases
    .filter((release) => release.isDraft && releaseTag.test(release.tagName))
    .map((release) => release.tagName)
    .sort(compareTags);
  if (tags.length === 0) return { status: "none" };
  if (tags.length === 1) return { status: "pending", tag: tags[0] as string };
  return { status: "multiple", tags };
}

/** Human-readable explanation for the workflow log and step summary. */
export function describePending(state: PendingRelease): string {
  if (state.status === "none") return "No release is pending.";
  if (state.status === "pending") {
    return [
      `Release ${state.tag} is still pending.`,
      "No new release PR will be created.",
      "",
      "Resume it with:",
      `Release → Run workflow → ${state.tag}`,
    ].join("\n");
  }
  return [
    `Several draft releases are pending: ${state.tags.join(", ")}.`,
    "Normally at most one can exist, so no new release PR will be created.",
    "",
    "Reconcile them: resume the one that matches main",
    `(Release → Run workflow → ${state.tags[state.tags.length - 1]}) and delete the superseded drafts.`,
  ].join("\n");
}

export const expectedAssets = (tag: string): string[] => [
  ...targets.map(
    (target) =>
      `entwine-${tag}-${target.triple}.${target.os === "win32" ? "zip" : "tar.gz"}`,
  ),
  "SHA256SUMS",
];

export const releasePackages = (): string[] => [
  "@entwine/cli",
  ...targets.map(platformPackage),
];

export interface ResumeFacts {
  /** Names of packages already visible at the release version. */
  published: readonly string[];
  /** Names of assets already attached to the draft. */
  assets: readonly string[];
  tag: string;
}

export interface ResumePlan {
  missingPackages: string[];
  missingAssets: string[];
  /** True when native binaries must be (re)built; false when everything exists. */
  needsBuild: boolean;
}

/** What does a resumed draft still need? Everything already done is reused. */
export function planResume(facts: ResumeFacts): ResumePlan {
  const missingPackages = releasePackages().filter(
    (name) => !facts.published.includes(name),
  );
  const missingAssets = expectedAssets(facts.tag).filter(
    (name) => !facts.assets.includes(name),
  );
  return {
    missingPackages,
    missingAssets,
    needsBuild: missingPackages.length > 0 || missingAssets.length > 0,
  };
}

function setOutput(values: Record<string, string>): void {
  // biome-ignore lint/suspicious/noUndeclaredEnvVars: GitHub Actions runner variable
  const file = process.env.GITHUB_OUTPUT;
  const lines = Object.entries(values).map(([k, v]) => `${k}=${v}`);
  if (file) appendFileSync(file, `${lines.join("\n")}\n`);
  else console.log(lines.join("\n"));
}

function summary(markdown: string): void {
  // biome-ignore lint/suspicious/noUndeclaredEnvVars: GitHub Actions runner variable
  const file = process.env.GITHUB_STEP_SUMMARY;
  if (file) appendFileSync(file, `${markdown}\n`);
}

async function main(): Promise<void> {
  const [command, ...rest] = process.argv.slice(2);
  const { values } = parseArgs({
    args: rest,
    options: {
      releases: { type: "string" },
      assets: { type: "string" },
      tag: { type: "string" },
    },
  });
  if (command === "guard") {
    if (!values.releases) throw new Error("--releases <json file> is required");
    const releases = JSON.parse(
      readFileSync(values.releases, "utf8"),
    ) as ReleaseSummary[];
    const state = assessPendingReleases(releases);
    const text = describePending(state);
    if (state.status !== "none") {
      console.log(
        `::notice title=Release pending::${text.replaceAll("\n", "%0A")}`,
      );
      summary(`### Release pending\n\n\`\`\`\n${text}\n\`\`\``);
    }
    console.log(text);
    setOutput({
      pending: state.status,
      tags:
        state.status === "none"
          ? ""
          : state.status === "pending"
            ? state.tag
            : state.tags.join(" "),
    });
  } else if (command === "plan") {
    const { tag } = values;
    if (!tag || !releaseTag.test(tag) || !values.assets) {
      throw new Error(
        "--tag vX.Y.Z and --assets <json names file> are required",
      );
    }
    const version = tag.slice(1);
    const lookups = await Promise.all(
      releasePackages().map(async (name) => ({
        name,
        lookup: await lookupVersion(name, version),
      })),
    );
    const assets = JSON.parse(readFileSync(values.assets, "utf8")) as string[];
    const plan = planResume({
      published: lookups
        .filter((entry) => entry.lookup.state === "published")
        .map((entry) => entry.name),
      assets,
      tag,
    });
    console.log(JSON.stringify(plan, null, 2));
    setOutput({ needs_build: String(plan.needsBuild) });
  } else {
    throw new Error("Usage: release-state.ts <guard|plan> ...");
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error: unknown) => {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  });
}
