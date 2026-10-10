import assert from "node:assert/strict";
import { test } from "node:test";
import {
  type GitHubFetcher,
  verifyReleaseTagTarget,
} from "./github-release.ts";

const response =
  (status: number, body: unknown = {}): GitHubFetcher =>
  async () => ({ status, json: async () => body });

test("an absent tag is safe to resume", async () => {
  await verifyReleaseTagTarget(
    "owner/repo",
    "v0.6.0",
    "commit-a",
    response(404),
  );
});

test("a release tag pointing to the expected commit can resume", async () => {
  await verifyReleaseTagTarget(
    "owner/repo",
    "v0.6.0",
    "commit-a",
    response(200, { object: { type: "commit", sha: "commit-a" } }),
  );
});

test("a tag pointing to another commit is rejected", async () => {
  await assert.rejects(
    verifyReleaseTagTarget(
      "owner/repo",
      "v0.6.0",
      "commit-a",
      response(200, { object: { type: "commit", sha: "commit-b" } }),
    ),
    /resolves to commit-b but the draft targets commit-a/,
  );
});

test("annotated tags are resolved to their target commit", async () => {
  let requests = 0;
  const fetcher: GitHubFetcher = async () => {
    requests++;
    return requests === 1
      ? {
          status: 200,
          json: async () => ({ object: { type: "tag", sha: "tag-object" } }),
        }
      : {
          status: 200,
          json: async () => ({ object: { type: "commit", sha: "commit-a" } }),
        };
  };
  await verifyReleaseTagTarget("owner/repo", "v0.6.0", "commit-a", fetcher);
  assert.equal(requests, 2);
});

test("tag verification fails closed when GitHub denies access", async () => {
  await assert.rejects(
    verifyReleaseTagTarget("owner/repo", "v0.6.0", "commit-a", response(403)),
    /Could not verify release tag \(HTTP 403\)/,
  );
});
