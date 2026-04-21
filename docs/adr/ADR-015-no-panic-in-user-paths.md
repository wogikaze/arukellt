# ADR-015: No-Panic Quality Standard for User-Facing Paths

ステータス: **DECIDED** — ユーザー到達パスでのpanic禁止  
**Created**: 2026-04-09  
**Scope**: CLI, LSP, extension, manifest parsing

## Context

When a user performs any normal operation — running `arukellt compile`, requesting
hover in the LSP, or activating the VS Code extension — they should never encounter
a Rust `panic!` or JavaScript-level crash. Panics produce confusing output with no
actionable guidance. They are unacceptable on any user-reachable code path.

## Decision

### 1. Definition of "user-reachable path"

A code path is user-reachable if it can be triggered by:

- Any `arukellt` CLI subcommand with valid or invalid arguments
- Any LSP request (hover, completion, diagnostics, etc.)
- Any VS Code extension activation, command, task execution, or debug adapter request
- Manifest parsing (`ark.toml`, `std/manifest.toml`, fixture manifests)

### 2. Banned patterns in user-reachable paths

- `panic!("...")` — use `anyhow::bail!` or return `Err(...)` instead
- `.unwrap()` on `Option` or `Result` where `None`/`Err` is user-reachable
- `.expect("...")` where the message is an internal programmer note, not a user message
- `todo!()`, `unimplemented!()` — replace with proper stubs or feature gates
- `unreachable!()` — only acceptable if logically unreachable by type invariant;
  if reachable via user input, convert to an error

### 3. Accepted patterns

- `.lock().unwrap()` on a `Mutex` — only panics if another thread already panicked;
  this is acceptable (mutex poison = already a bug)
- `.expect("invariant: ...")` where the invariant is verifiably upheld by the type
  system or a compiler-checked assertion
- `panic!` in `#[cfg(test)]` code
- `panic!` in code only reachable via `--internal-*` developer flags

### 4. Error output standard

When a user-facing error occurs:
- Print a clear human-readable message (no stack trace by default)
- Include the relevant context (which file, which command, which field)
- Exit with a non-zero code
- Suggest a fix or point to documentation where possible

### 5. New code rule

New PRs must not introduce `unwrap()`, `expect()`, `panic!()`, `todo!()`, or
`unimplemented!()` in user-reachable paths. Reviewers must reject PRs that do so.

## Current State (2026-04-09)

Audit of user-facing crates:

| Crate | Dangerous panics found |
|-------|----------------------|
| `crates/arukellt/src/` | None |
| `crates/ark-lsp/src/` | None (all `lock().unwrap()` are mutex-only) |
| `crates/ark-manifest/src/` | None |
| `crates/ark-driver/src/` | None |
| `extensions/arukellt-all-in-one/src/` | None |

The user-facing crates are clean as of this ADR.

## Enforcement

- `scripts/manager.py` includes a panic audit check: `--no-panic-audit`
- CI runs this check in the `integration` layer
- New violations are treated as `P1` bugs (immediate fix required)

## References

- `issues/done/243-no-panic-in-user-paths-quality-standard.md`
- `docs/contributing.md`
