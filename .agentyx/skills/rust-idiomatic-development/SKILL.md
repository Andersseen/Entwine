---
name: rust-idiomatic-development
description: Write and review idiomatic Rust with clear ownership, expressive types and small public APIs.
---

# Rust idiomatic development

Use when adding or changing Rust code: new types, functions, error handling, public API.

## Ownership first

- Borrow before you clone. Take `&str`/`&Path`/`&[T]` in parameters; return owned data only when the caller needs it. A `.clone()` added to silence the borrow checker is a design smell: restructure the borrow, or justify the clone.
- Prefer `impl AsRef<Path>` / `impl Into<String>` only at API edges where callers benefit; keep internals concrete.
- Avoid allocation in loops: reuse buffers, use iterators, `Cow<str>` when most inputs pass through unchanged.

## Model with types

- Make invalid states unrepresentable: an `enum` per distinct case instead of a `String` plus flags. Newtypes for identifiers and routes whose constructor validates.
- Match exhaustively; avoid `_ =>` on enums you own so new variants break the build, not behavior.
- Use iterator chains when they read as a pipeline (`filter_map`, `collect::<Result<_,_>>()`); use a `for` loop when there are side effects or early exits.

## Errors, not panics

- Library and CLI paths that touch user input, files or the network return `Result`. No `unwrap`/`expect`/indexing on data that originates outside the process; `expect` only for true invariants, with a message that states the invariant.
- Add context where the error crosses a boundary (which path, which key). Diagnostics are user-visible output: write them for the user.

## API and layering

- Keep `pub` minimal; prefer `pub(crate)`. Every public item is a promise.
- Separate wire/serde types from domain types; convert at the boundary and validate there. Do not put `#[derive(Deserialize)]` on a type just because it is convenient if it bypasses constructor validation.
- Comments explain intent and constraints, not syntax. Doc comments (`///`) on every public item, stating failure modes.

## Safety policy

- `unsafe` is a per-repository policy. Check `[workspace.lints]` / `Cargo.toml` first. Entwine sets `unsafe_code = "forbid"`: never add `unsafe`, never `#[allow(unsafe_code)]`.

## Verify

Run the `cargo-workspace-verification` gate. For a changed signature, also `cargo doc --workspace --no-deps` if docs reference it.
