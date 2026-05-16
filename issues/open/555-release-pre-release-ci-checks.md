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

## Recheck — 2026-05-16

Current command evidence:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --exclude ark-llvm -- -D warnings` passes clean
  (same result).
- `python3 scripts/manager.py verify quick` passes 22/22 (same result).
- `python3 scripts/manager.py verify component` **NOW PASSES 101/101**
  (improved from previous recheck where it failed all 6 component interop
  smoke tests with `error[E0500|emit]: unsupported emit mode: component`).
- `cargo test --workspace --exclude ark-llvm`: fixture harness still RED.
  Latest observed: `PASS: 0 FAIL: 414 SKIP: 0` (similar to previous recheck
  which reported `PASS: 413 FAIL: 406 SKIP: 20`; the ratio changed due to
  target-scope changes).
- `bash scripts/run/test-opt-equivalence.sh`: **SCRIPT DOES NOT EXIST.**
  - Neither `scripts/run/test-opt-equivalence.sh` nor any `opt-equiv` variant
    exists under `scripts/`.
  - The manager.py `--opt-equiv` flag is listed in the `_MISSING` set
    (unimplemented flags), so no replacement gate exists either.
  - The release-checklist.md references `bash scripts/manager.py --opt-equiv`
    which is also unimplemented.
- `cargo test -p ark-lsp --lib`: **ark-lsp package RETIRED** (removed in #572).
  The release-checklist.md has been updated to note that selfhost LSP coverage
  lives under `python3 scripts/manager.py verify quick`.

Updated acceptance checklist:

- [ ] `cargo test --workspace --exclude ark-llvm` passes — **FAIL** (414 fixture failures)
- [ ] `cargo test -p arukellt --test harness` passes (all fixtures green) — **FAIL** (414 failures)
- [x] `cargo clippy --workspace --exclude ark-llvm -- -D warnings` clean — **PASS**
- [x] `cargo fmt --all -- --check` clean — **PASS**
- [x] `python scripts/manager.py verify quick` passes — **PASS** (22/22)
- [x] `python scripts/manager.py verify component` passes — **PASS** (101/101) [IMPROVED]
- [ ] `bash scripts/run/test-opt-equivalence.sh` passes — **MISSING** (script not found; opt-equiv not implemented in manager.py)
- [ ] LSP unit tests: `cargo test -p ark-lsp --lib` passes — **RETIRED** (ark-lsp removed in #572)

Updated verdict: close-candidate `no`. Notable improvement: component interop now
passes 101/101. However, the fixture harness remains RED (414 failures), the
opt-equiv script does not exist, and the `ark-lsp` test command is retired.
The acceptance criteria in this issue need to be updated to match the current
release-checklist.md (which now uses `python3 scripts/manager.py verify quick`
for LSP coverage and `bash scripts/manager.py --opt-equiv` for opt-equiv, though
the latter is also unimplemented).

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
