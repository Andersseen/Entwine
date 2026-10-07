/**
 * npm registry visibility helpers for release verification.
 *
 * A publish that exited successfully is not instantly visible everywhere: the
 * registry's CDN may keep answering "not found" for a short while. These helpers
 * ask the registry directly (bypassing local npm caches) and bound the waiting.
 */

export type VersionState = "published" | "absent" | "unknown";

export interface VersionLookup {
  state: VersionState;
  detail: string;
}

export type Fetcher = (
  url: string,
  init?: { headers?: Record<string, string> },
) => Promise<{
  status: number;
  json(): Promise<unknown>;
}>;

export const registryUrl = (name: string, version: string): string =>
  `https://registry.npmjs.org/${name.replace("/", "%2F")}/${encodeURIComponent(version)}`;

/** Ask the registry whether this exact `name@version` exists. Never throws. */
export async function lookupVersion(
  name: string,
  version: string,
  fetcher: Fetcher = fetch as unknown as Fetcher,
  nonce: string = String(Date.now()),
): Promise<VersionLookup> {
  const url = `${registryUrl(name, version)}?entwine-check=${nonce}`;
  try {
    const response = await fetcher(url, {
      headers: { accept: "application/json", "cache-control": "no-cache" },
    });
    if (response.status === 404) {
      return { state: "absent", detail: `${name}@${version}: HTTP 404` };
    }
    if (response.status !== 200) {
      return {
        state: "unknown",
        detail: `${name}@${version}: HTTP ${response.status}`,
      };
    }
    const body = (await response.json()) as { name?: string; version?: string };
    if (body.name === name && body.version === version) {
      return { state: "published", detail: `${name}@${version}` };
    }
    return {
      state: "unknown",
      detail: `${name}@${version}: unexpected metadata ${body.name}@${body.version}`,
    };
  } catch (error) {
    return {
      state: "unknown",
      detail: `${name}@${version}: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}

export interface Backoff {
  initialMs: number;
  maxMs: number;
  factor: number;
}

/** Delay before retry number `attempt` (1-based), capped. */
export function backoffDelay(attempt: number, policy: Backoff): number {
  const raw = policy.initialMs * policy.factor ** Math.max(0, attempt - 1);
  return Math.min(policy.maxMs, Math.round(raw));
}

export interface WaitOptions {
  timeoutMs: number;
  backoff: Backoff;
  lookup?: (name: string, version: string) => Promise<VersionLookup>;
  sleep?: (ms: number) => Promise<void>;
  now?: () => number;
  log?: (message: string) => void;
}

export interface WaitResult {
  visible: string[];
  /** Registry answered 404 for these when the deadline passed. */
  absent: string[];
  /** Registry could not be asked (network, 5xx) for these. */
  unknown: string[];
}

/**
 * Poll until every package is visible at `version`, or the deadline passes.
 * Result distinguishes "registry says no" from "registry unreachable".
 */
export async function waitForVersions(
  names: readonly string[],
  version: string,
  options: WaitOptions,
): Promise<WaitResult> {
  const lookup = options.lookup ?? ((n, v) => lookupVersion(n, v));
  const sleep =
    options.sleep ??
    ((ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms)));
  const now = options.now ?? Date.now;
  const log = options.log ?? (() => {});
  const deadline = now() + options.timeoutMs;
  const visible = new Set<string>();
  let latest = new Map<string, VersionLookup>();
  for (let attempt = 1; ; attempt++) {
    latest = new Map();
    for (const name of names) {
      if (visible.has(name)) continue;
      const result = await lookup(name, version);
      if (result.state === "published") visible.add(name);
      else latest.set(name, result);
    }
    if (visible.size === names.length) break;
    const delay = backoffDelay(attempt, options.backoff);
    if (now() + delay > deadline) break;
    log(
      `Waiting for the registry: ${[...latest.values()].map((l) => l.detail).join("; ")} (retry in ${Math.round(delay / 1000)}s)`,
    );
    await sleep(delay);
  }
  const pending = names.filter((n) => !visible.has(n));
  return {
    visible: names.filter((n) => visible.has(n)),
    absent: pending.filter((n) => latest.get(n)?.state === "absent"),
    unknown: pending.filter((n) => latest.get(n)?.state !== "absent"),
  };
}
