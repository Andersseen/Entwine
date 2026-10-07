/** Fixture layout shared by global setup and the specs. */
import { tmpdir } from "node:os";
import { join } from "node:path";

export const ports = { full: 4173, private: 4174 } as const;
export const origins = {
  full: `http://127.0.0.1:${ports.full}`,
  private: `http://127.0.0.1:${ports.private}`,
};
export const work = join(tmpdir(), "entwine-e2e");
export const fixtureRoot = (name: "full" | "private"): string =>
  join(work, name);
/** Unique text planted in agent files of the private fixture; it must never be published. */
export const canary = "ENTWINE-PRIVATE-CANARY-9d41c7";
