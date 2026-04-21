# Release: Pre-Release CI Checks

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure pre-release CI checks pass for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [ ] `cargo test --workspace --exclude ark-llvm` passes
- [ ] `cargo test -p arukellt --test harness` passes (all fixtures green)
- [ ] `cargo clippy --workspace --exclude ark-llvm -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean
- [ ] `bash scripts/run/verify-harness.sh --quick` passes
- [ ] `bash scripts/run/verify-harness.sh --component` passes (component interop)
- [ ] `bash scripts/run/verify-harness.sh --opt-equiv` passes (O0 == O1)
- [ ] LSP unit tests: `cargo test -p ark-lsp --lib` passes

## Required Verification

- Run full test suite (excluding ark-llvm)
- Run harness tests with all fixtures
- Run clippy with warnings as errors
- Check code formatting
- Run verify-harness quick checks
- Run verify-harness component interop
- Run verify-harness optimization equivalence
- Run LSP unit tests

## Close Gate

All pre-release CI checks must pass without warnings or errors.

## Primary Paths

- Test suite configuration
- Clippy configuration
- Formatting configuration
- Verify harness scripts
- LSP unit test suite

## Non-Goals

- ark-llvm testing (excluded from scope)
