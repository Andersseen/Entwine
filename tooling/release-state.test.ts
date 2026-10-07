import assert from "node:assert/strict";
import { test } from "node:test";
import { publishIfMissing } from "./publish-npm.ts";
import {
  backoffDelay,
  type Fetcher,
  lookupVersion,
  waitForVersions,
} from "./registry.ts";
import {
  assessPendingReleases,
  describePending,
  expectedAssets,
  planResume,
  releasePackages,
} from "./release-state.ts";

const draft = (tagName: string) => ({ tagName, isDraft: true });
const published = (tagName: string) => ({ tagName, isDraft: false });

test("no drafts means release-please may run", () => {
  assert.deepEqual(assessPendingReleases([]), { status: "none" });
  assert.deepEqual(
    assessPendingReleases([published("v0.1.0"), published("v0.2.0")]),
    { status: "none" },
  );
});

test("one draft blocks release-please and names the tag", () => {
  const state = assessPendingReleases([published("v0.2.0"), draft("v0.5.0")]);
  assert.deepEqual(state, { status: "pending", tag: "v0.5.0" });
  const text = describePending(state);
  assert.match(text, /Release v0\.5\.0 is still pending\./);
  assert.match(text, /No new release PR will be created\./);
  assert.match(text, /Run workflow → v0\.5\.0/);
});

test("several drafts are an invalid state and are all listed, never guessed", () => {
  const state = assessPendingReleases([
    draft("v0.10.0"),
    draft("v0.2.1"),
    draft("v0.5.0"),
  ]);
  assert.deepEqual(state, {
    status: "multiple",
    tags: ["v0.2.1", "v0.5.0", "v0.10.0"],
  });
  assert.match(describePending(state), /v0\.2\.1, v0\.5\.0, v0\.10\.0/);
});

test("drafts that are not Entwine version releases are ignored", () => {
  assert.deepEqual(
    assessPendingReleases([draft("notes"), draft("untagged-abc123")]),
    { status: "none" },
  );
});

test("regression: draft stays blocking after an unrelated push, closes once published", () => {
  // feature -> release PR merged -> draft created, public verification failed
  const afterFailure = [published("v0.2.0"), draft("v0.5.0")];
  // unrelated/fix push lands on main: the guard still sees the draft
  assert.equal(assessPendingReleases(afterFailure).status, "pending");
  // draft resumed and published: tag exists, loop closed
  const afterResume = [published("v0.2.0"), published("v0.5.0")];
  assert.equal(assessPendingReleases(afterResume).status, "none");
});

test("resume reuses published packages and attached assets", () => {
  const tag = "v0.5.0";
  const full = planResume({
    published: releasePackages(),
    assets: expectedAssets(tag),
    tag,
  });
  assert.equal(full.needsBuild, false);
  const partial = planResume({
    published: releasePackages().slice(1),
    assets: expectedAssets(tag).slice(1),
    tag,
  });
  assert.equal(partial.needsBuild, true);
  assert.deepEqual(partial.missingPackages, ["@entwine/cli"]);
  assert.equal(partial.missingAssets.length, 1);
});

test("expected assets cover every platform plus checksums", () => {
  const assets = expectedAssets("v1.2.3");
  assert.ok(assets.includes("entwine-v1.2.3-x86_64-pc-windows-msvc.zip"));
  assert.ok(assets.includes("entwine-v1.2.3-aarch64-apple-darwin.tar.gz"));
  assert.ok(assets.includes("SHA256SUMS"));
  assert.equal(assets.length, 5);
});

const reply =
  (status: number, body: unknown = {}): Fetcher =>
  async () => ({ status, json: async () => body });

test("lookup classifies registry answers", async () => {
  const ok = await lookupVersion(
    "@entwine/cli",
    "1.0.0",
    reply(200, { name: "@entwine/cli", version: "1.0.0" }),
  );
  assert.equal(ok.state, "published");
  assert.equal(
    (await lookupVersion("@entwine/cli", "1.0.0", reply(404))).state,
    "absent",
  );
  assert.equal(
    (await lookupVersion("@entwine/cli", "1.0.0", reply(503))).state,
    "unknown",
  );
  assert.equal(
    (
      await lookupVersion(
        "@entwine/cli",
        "1.0.0",
        reply(200, { name: "other", version: "1.0.0" }),
      )
    ).state,
    "unknown",
  );
  const broken = await lookupVersion("@entwine/cli", "1.0.0", async () => {
    throw new Error("socket hang up");
  });
  assert.equal(broken.state, "unknown");
  assert.match(broken.detail, /socket hang up/);
});

test("lookup escapes scoped names and busts caches", async () => {
  let seen = "";
  await lookupVersion(
    "@entwine/cli",
    "1.0.0",
    async (url) => {
      seen = url;
      return { status: 404, json: async () => ({}) };
    },
    "n1",
  );
  assert.equal(
    seen,
    "https://registry.npmjs.org/@entwine%2Fcli/1.0.0?entwine-check=n1",
  );
});

test("backoff grows and is capped", () => {
  const policy = { initialMs: 1000, maxMs: 5000, factor: 2 };
  assert.deepEqual(
    [1, 2, 3, 4, 5].map((n) => backoffDelay(n, policy)),
    [1000, 2000, 4000, 5000, 5000],
  );
});

test("waiting succeeds once propagation catches up", async () => {
  let calls = 0;
  let clock = 0;
  const result = await waitForVersions(["a", "b"], "1.0.0", {
    timeoutMs: 60_000,
    backoff: { initialMs: 1000, maxMs: 1000, factor: 1 },
    now: () => clock,
    sleep: async (ms) => {
      clock += ms;
    },
    lookup: async (name) => {
      calls++;
      const visible = name === "a" || clock >= 3000;
      return { state: visible ? "published" : "absent", detail: name };
    },
  });
  assert.deepEqual(result.visible, ["a", "b"]);
  assert.deepEqual(result.absent, []);
  assert.ok(calls > 2);
});

test("waiting is finite and separates absent from unreachable", async () => {
  let clock = 0;
  const result = await waitForVersions(["gone", "flaky", "ok"], "1.0.0", {
    timeoutMs: 10_000,
    backoff: { initialMs: 4000, maxMs: 4000, factor: 1 },
    now: () => clock,
    sleep: async (ms) => {
      clock += ms;
    },
    lookup: async (name) => ({
      state:
        name === "ok" ? "published" : name === "gone" ? "absent" : "unknown",
      detail: name,
    }),
  });
  assert.deepEqual(result.visible, ["ok"]);
  assert.deepEqual(result.absent, ["gone"]);
  assert.deepEqual(result.unknown, ["flaky"]);
  assert.ok(clock <= 10_000);
});

test("publishing is idempotent", async () => {
  let published = 0;
  const publish = () => {
    published++;
  };
  assert.equal(
    await publishIfMissing("p", "1.0.0", async () => true, publish),
    "skipped",
  );
  assert.equal(published, 0);
  let visible = false;
  assert.equal(
    await publishIfMissing(
      "p",
      "1.0.0",
      async () => visible,
      () => {
        visible = true;
        publish();
      },
    ),
    "published",
  );
  assert.equal(published, 1);
});

test("a failed publish that actually landed is not an error", async () => {
  let calls = 0;
  const outcome = await publishIfMissing(
    "p",
    "1.0.0",
    async () => ++calls > 1,
    () => {
      throw new Error("timeout after upload");
    },
  );
  assert.equal(outcome, "skipped");
});

test("a genuinely failed publish is reported", async () => {
  await assert.rejects(
    publishIfMissing(
      "p",
      "1.0.0",
      async () => false,
      () => {
        throw new Error("E403");
      },
    ),
    /npm publish failed for p@1\.0\.0: E403/,
  );
});
