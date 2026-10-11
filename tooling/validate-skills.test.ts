import assert from "node:assert/strict";
import { test } from "node:test";
import { commandSpans, problemsForSpan } from "./validate-skills.ts";

const help = (args: string[]): string =>
  args.length === 0
    ? "Commands:\n  check  Validate\n  setup  Configure\n"
    : args[0] === "setup"
      ? "Options:\n  --dry-run\n  --provider <P>\n"
      : "Options:\n";

test("extracts entwine command spans and ignores negations and npx prefixes", () => {
  const md = [
    "Run `entwine check` then `npx --yes @entwine/cli setup --dry-run`.",
    "There is no `entwine publish`.",
    "Not code: entwine.toml",
  ].join("\n");
  assert.deepEqual(commandSpans(md), [
    "entwine check",
    "entwine setup --dry-run",
  ]);
});

test("flags unknown commands and flags", () => {
  assert.deepEqual(problemsForSpan("entwine setup --dry-run", help), []);
  assert.equal(problemsForSpan("entwine deploy", help).length, 1);
  assert.equal(problemsForSpan("entwine setup --force", help).length, 1);
  assert.deepEqual(problemsForSpan("entwine --version", help), []);
});
