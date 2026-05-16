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

- Build release binary: `cargo build --release -p arukellt`
- Run smoke tests against release binary
- Verify exit codes and outputs match expectations

## Current Evidence

Verified against `target/release/arukellt` on 2026-05-14 after
`cargo build --release -p arukellt` completed successfully.

- `target/release/arukellt --version` exits 0 and prints `arukellt 0.1.0`.
- `target/release/arukellt run tests/fixtures/hello_world.ark` exits 0 and
  prints `Hello, World!`.
- `target/release/arukellt check tests/fixtures/type_error.diag` exits 1 and
  reports `error[E0001|lex]: 4 lexer error(s)`.

## Close Gate

All three smoke tests pass with the release binary.

## Primary Paths

- `Cargo.toml` (binary package configuration)
- `src/main.rs` (CLI entrypoint)
- `tests/fixtures/hello_world.ark` (smoke test fixture)
- `tests/fixtures/type_error.diag` (smoke test fixture)

## Non-Goals

- Performance optimization
- Feature completeness
- Cross-platform testing (linux-x86_64 only for now)
