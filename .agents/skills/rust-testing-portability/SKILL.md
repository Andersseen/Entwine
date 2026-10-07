---
name: rust-testing-portability
description: Write deterministic, portable Rust tests that cover regressions and user-visible diagnostics.
---

# Rust testing and portability

Use when adding tests for Rust code, fixing a bug, or touching code that runs on Linux, macOS and Windows.

## Choose the level

- Unit tests (`#[cfg(test)] mod tests`) for pure logic and edge cases in one module.
- Integration tests (`crates/*/tests/*.rs`) for behavior through the public API or the real CLI binary (`env!("CARGO_BIN_EXE_<name>")` / `assert_cmd`-style), including exit codes and stderr/stdout text users see.
- A fixed bug always gets a regression test that fails without the fix. Name it for the behavior, not the issue number.

## Deterministic

- Use `tempfile::tempdir()`; never write into the source tree or rely on the current directory. Build fixtures in code or small checked-in directories, not by copying the repository.
- No wall-clock sleeps. Wait on a condition with a bounded retry (poll a port/file) or make the code injectable. Never assume a fixed port; bind port 0.
- Sort before comparing anything that comes from a directory walk or a hash map.
- Do not depend on environment: set the variables the test needs on the `Command`, clear the ones it must not inherit.

## Portable

- Build paths with `join`; compare normalized `/` strings only where the code under test promises them.
- Gate Unix-only behavior with `#[cfg(unix)]` (symlinks, permissions) and Windows-only with `#[cfg(windows)]`; add a Windows-separator case (`a\\b.md`) for any route/ID normalization.
- Don't assume the executable name: `entwine.exe` on Windows. Don't assume `\n` in files written by tools; normalize `\r\n` before snapshot comparison.
- Case-insensitive filesystems: do not create two fixtures differing only by case in one test directory.

## Snapshots

Keep golden files intentional: small, reviewed, refreshed only through the project's documented command, and free of paths, timestamps, versions and absolute directories.

## Verify

`cargo test --workspace --locked`. For flakiness, re-run the one test in a loop (`for i in $(seq 20); do cargo test -p <crate> <name> || break; done`) before claiming it is stable.
