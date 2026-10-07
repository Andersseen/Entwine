---
name: rust-cli-filesystem-safety
description: Handle untrusted paths, symlinks and output directories safely in Rust command-line tools.
---

# Rust CLI filesystem safety

Use when code reads, walks, copies or writes files whose names or contents come from the repository or user: scanners, generators, scaffolding, dev servers.

## Treat input as hostile

Repository paths, file names, front matter, links and `SKILL.md`-style manifests are untrusted. A file inside the project can still point outside it (`..`, absolute paths, symlinks, junctions, Windows drive prefixes, UNC paths, reserved device names).

## Rules

- Use `Path`/`PathBuf` and `Component` iteration for path logic, never string slicing or `split('/')` on OS paths. Convert to `/`-joined strings only for URLs/IDs, replacing `\\` explicitly.
- Validate relative paths component-wise: accept only `Component::Normal`; reject `..`, `RootDir`, `Prefix`, empty.
- Containment: `canonicalize` both root and target, then `starts_with(root)` (component-wise, not string prefix). Do this before reading the content, and again for anything resolved from a link target.
- Symlinks: decide per feature, default to refuse. Walk with `follow_links(false)`; check `symlink_metadata`, not `metadata`. Never create or write through a symlinked directory.
- Case-insensitive filesystems (macOS, Windows): two routes/files differing only by case collide. Detect collisions on a lower-cased key before writing. Also watch trailing dots/spaces and reserved names on Windows.
- Output: build into a staging directory (a sibling temp dir on the same filesystem), then rename into place; on failure keep the last good output. Never delete a directory you did not create or cannot prove is yours (check for a marker file).
- Preserve user files: create-new with `OpenOptions::create_new(true)`; report `Exists` rather than overwriting.
- Bound work: max depth, max entries, max file size. Skip with a warning diagnostic rather than failing the whole run when one file is unreadable or not UTF-8.
- No panics on path failures: non-UTF-8 names (`to_str()` is `Option`), permission errors, TOCTOU races (a file vanishing between listing and reading) are normal.

## Verify

- Add a test per rule you touch: traversal strings (`../x`, `a/../../x`, `/abs`, `C:\\x`), a symlink to outside (`#[cfg(unix)]`), case-collision pair, unreadable/non-UTF-8 file.
- Run the `rust-testing-portability` checklist for Windows separators.
