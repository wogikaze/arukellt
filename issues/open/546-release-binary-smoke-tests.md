# Release: Binary Smoke Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure binary smoke tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [ ] `arukellt --version` exits 0
- [ ] `arukellt run tests/fixtures/hello_world.ark` outputs `Hello, World!`
- [ ] `arukellt check tests/fixtures/type_error.diag` exits non-zero

## Required Verification

- Build release binary: `cargo build --release -p arukellt`
- Run smoke tests against release binary
- Verify exit codes and outputs match expectations

## Close Gate

All three smoke tests must pass with the release binary.

## Primary Paths

- `Cargo.toml` (binary package configuration)
- `src/main.rs` (CLI entrypoint)
- `tests/fixtures/hello_world.ark` (smoke test fixture)
- `tests/fixtures/type_error.diag` (smoke test fixture)

## Non-Goals

- Performance optimization
- Feature completeness
- Cross-platform testing (linux-x86_64 only for now)
