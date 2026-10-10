export interface GitHubResponse {
  status: number;
  json(): Promise<unknown>;
}

export type GitHubFetcher = (
  url: string,
  init?: { headers?: Record<string, string> },
) => Promise<GitHubResponse>;

interface GitObject {
  type?: string;
  sha?: string;
}

interface GitRef {
  object?: GitObject;
}

interface AnnotatedTag {
  object?: GitObject;
}

const api = "https://api.github.com";

async function readTagTarget(
  repository: string,
  object: GitObject,
  fetcher: GitHubFetcher,
  depth = 0,
): Promise<string> {
  if (!object.sha || !object.type) {
    throw new Error("GitHub returned an incomplete tag target");
  }
  if (object.type === "commit") return object.sha;
  if (object.type !== "tag" || depth >= 8) {
    throw new Error(`Unsupported Git tag target type: ${object.type}`);
  }

  const response = await fetcher(
    `${api}/repos/${repository}/git/tags/${encodeURIComponent(object.sha)}`,
  );
  if (response.status !== 200) {
    throw new Error(
      `Could not resolve annotated tag target (HTTP ${response.status})`,
    );
  }
  const body = (await response.json()) as AnnotatedTag;
  if (!body.object)
    throw new Error("GitHub returned an incomplete annotated tag");
  return readTagTarget(repository, body.object, fetcher, depth + 1);
}

/** Verify an existing release tag before resuming; only a genuine 404 means absent. */
export async function verifyReleaseTagTarget(
  repository: string,
  tag: string,
  expectedCommit: string,
  fetcher: GitHubFetcher,
): Promise<void> {
  const response = await fetcher(
    `${api}/repos/${repository}/git/ref/tags/${encodeURIComponent(tag)}`,
  );
  if (response.status === 404) return;
  if (response.status !== 200) {
    throw new Error(`Could not verify release tag (HTTP ${response.status})`);
  }

  const ref = (await response.json()) as GitRef;
  if (!ref.object) throw new Error("GitHub returned an incomplete tag ref");
  const actualCommit = await readTagTarget(repository, ref.object, fetcher);
  if (actualCommit.toLowerCase() !== expectedCommit.toLowerCase()) {
    throw new Error(
      `Tag ${tag} resolves to ${actualCommit} but the draft targets ${expectedCommit}`,
    );
  }
}

export async function verifyReleaseTagTargetFromEnvironment(
  repository: string,
  tag: string,
  expectedCommit: string,
): Promise<void> {
  // biome-ignore lint/suspicious/noUndeclaredEnvVars: GitHub Actions runner variable
  const token = process.env.GH_TOKEN;
  if (!token) throw new Error("GH_TOKEN is required to verify the release tag");
  await verifyReleaseTagTarget(repository, tag, expectedCommit, (url) =>
    fetch(url, {
      headers: {
        accept: "application/vnd.github+json",
        authorization: `Bearer ${token}`,
        "x-github-api-version": "2022-11-28",
      },
    }),
  );
}
