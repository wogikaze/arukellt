# Release: Binary Smoke Tests

> **Status:** done
> **Track:** release
> **Type:** Verification

## Scope

Ensure binary smoke tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [x] `arukellt --version` exits 0
- [x] `arukellt run tests/fixtures/hello_world.ark` outputs `Hello, World!`
- [x] `arukellt check tests/fixtures/type_error.diag` exits non-zero

## Required Verification

- Prepare the release CLI entrypoint wrapper as `target/release/arukellt`
- Run smoke tests against the release CLI entrypoint
- Verify exit codes and outputs match expectations

## Current Evidence

Verified against `target/release/arukellt` on 2026-05-14.

- `target/release/arukellt --version` exits 0 and prints `arukellt 0.1.0`.
- `target/release/arukellt run tests/fixtures/hello_world.ark` exits 0 and
  prints `Hello, World!`.
- `target/release/arukellt check tests/fixtures/type_error.diag` exits 1 and
  reports `error[E0001|lex]: 4 lexer error(s)`.

## Recheck — 2026-05-17

The workspace no longer has an `arukellt` Rust binary crate. The release CLI
entrypoint is the selfhost wrapper copied to `target/release/arukellt`, matching
the packaging job in `.github/workflows/ci.yml`.

Current release smoke coverage is enforced by CI:

- Integration job prepares `target/release/arukellt` from
  `scripts/run/arukellt-selfhost.sh` and runs `--help`, `--version`, and
  `docs/examples/hello.ark` on both T3 primary and T1 supported paths.
- Integration job also runs the release checklist fixtures:
  `tests/fixtures/hello_world.ark` must print `Hello, World!`, and
  `tests/fixtures/type_error.diag` must fail `check`.
- Packaging job prepares the same release CLI entrypoint and runs `--help` /
  `--version`.
- `scripts/run/arukellt-selfhost.sh` now completes the selfhost `run` command by
  executing the generated wasm via `wasmtime`, so the release CLI entrypoint
  matches the documented `arukellt run <file>` behavior.

Updated verdict: close-candidate `yes` for the current selfhost-wrapper release
entrypoint.

## Close Gate

All three smoke tests pass with the release binary.

## Primary Paths

- `scripts/run/arukellt-selfhost.sh` (release CLI entrypoint wrapper)
- `.github/workflows/ci.yml`
- `tests/fixtures/hello_world.ark` (smoke test fixture)
- `tests/fixtures/type_error.diag` (smoke test fixture)

## Non-Goals

- Performance optimization
- Feature completeness
- Cross-platform testing (linux-x86_64 only for now)
