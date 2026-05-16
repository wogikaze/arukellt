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
- [ ] `python scripts/manager.py verify quick` passes
- [ ] `python scripts/manager.py verify component` passes (component interop)
- [ ] `bash scripts/run/test-opt-equivalence.sh` passes (O0 == O1)
- [ ] LSP unit tests: `cargo test -p ark-lsp --lib` passes

## Recheck — 2026-05-14

Current command evidence:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace -- -D warnings` initially failed on one
  `needless_borrows_for_generic_args` warning in `crates/arukellt/src/main.rs`;
  the warning was fixed and the command now passes.
- `python3 scripts/manager.py verify quick` passes 22/22.
- `cargo test --workspace --exclude ark-llvm` runs unit tests successfully, then
  fails at the `arukellt` fixture harness. Cargo also warns that `ark-llvm` is
  not present in the current workspace.
- `cargo test -p arukellt --test harness` / full fixture execution remains red:
  latest observed summary is `PASS: 413 FAIL: 406 SKIP: 20`.
- `python3 scripts/manager.py verify component` fails all 6 component interop
  smoke tests because selfhost currently reports
  `error[E0500|emit]: unsupported emit mode: component`.
- The old `cargo test -p ark-lsp --lib` acceptance item is no longer a valid
  command after Rust `ark-lsp` retirement; current selfhost LSP coverage lives
  under `python3 scripts/manager.py verify quick`.

Updated verdict: close-candidate `no`. The formatting, clippy, and quick gates
are green, but the full fixture harness and component interop gates are still
red, and the checklist still contains a retired `ark-lsp` command.

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
