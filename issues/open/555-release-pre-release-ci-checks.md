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

## Recheck — 2026-05-16 (final)

Current command evidence (all checks re-executed):

- `cargo fmt --all -- --check` — **PASS** (no output).
- `cargo clippy --workspace -- -D warnings` — **FAIL** (4 dead_code errors in
  `crates/ark-resolve/src/manifest.rs`): `severity_for` method, `LintLevel` enum,
  `MissingField` variant, and `discover_path_dependency_roots`/`require_bin`
  associated items are all unreferenced.
- `python3 scripts/manager.py verify quick` — **FAIL** (21/23, 2 failures):
  1. 4 doc examples fail in `docs/design/lang-uplift-gap-ledger.md` (blocks 0, 2, 3)
     and `docs/language/spec.md` (block 2) — these contain code using language
     syntax no longer recognized by the current compiler.
  2. Broken internal links (`bash scripts/check/check-links.sh` fails with a
     syntax error in the script itself at line 24).
- `python3 scripts/manager.py verify component` — **FAIL** (79/101, 22 failures).
  All 22 failures are in the "wasmtime" runner and follow a pattern: primitives
  with renamed exports (bool, char, f64, i8, i16, i32, i64, u8, u16, u32, u64)
  plus string-count64, string-countu64, int-widths, metadata-names,
  metadata-scalars, multi-type-exports, and primitives-float.
- `python3 scripts/manager.py verify fixtures` — **PASS** (48 pass, 0 fail, 344 skip).
  Selfhost fixture parity is clean. The old Rust `arukellt` binary crate no longer
  has a Cargo.toml, so `cargo test -p arukellt --test harness` is dead.
- `python3 scripts/manager.py verify --selfhost-parity` — **FAIL** (diag parity:
  23 pass, 27 skip, 3 fail). Three trait-related diagnostics regressed:
  `trait_unresolved_var_bound.ark`, `trait_overlapping_impl.ark`,
  `trait_ambiguous_bound.ark`.
- `cargo test --workspace` — **PASS** (all unit tests pass; no harness since
  `arukellt` binary crate is a directory stub without Cargo.toml).
- `bash scripts/manager.py --opt-equiv` — **NEVER EXISTED**. The `--opt-equiv`
  flag has always been in the `_MISSING` set of `manager.py`. No implementation
  exists in any form. Removed from acceptance criteria.
- `cargo test -p ark-lsp --lib` — **RETIRED** (#572). Removed from acceptance criteria.

### Updated acceptance criteria (current reality)

Updated to match what is actually verifiable in the current codebase, aligned with
the release-checklist.md after its 2026-05-16 update:

- [ ] `cargo clippy --workspace -- -D warnings` clean — **FAIL** (4 dead_code in ark-resolve)
- [x] `cargo fmt --all -- --check` clean — **PASS**
- [ ] `python3 scripts/manager.py verify quick` passes — **FAIL** (21/23; doc examples + broken links)
- [ ] `python3 scripts/manager.py verify component` passes — **FAIL** (79/101; 22 renamed-export failures)
- [x] `python3 scripts/manager.py verify fixtures` passes — **PASS** (48/48)
- [ ] `python3 scripts/manager.py verify --selfhost-parity` passes — **FAIL** (3 diagnostic regressions)
- [x] `cargo test --workspace` passes — **PASS** (unit tests only)
- [ ] opt-equiv (O0 == O1) — **NOT IMPLEMENTED** (no acceptance check; flagged in release-checklist)

Updated verdict: **close-candidate `no`**. Of the 8 acceptance criteria, only
3 pass (fmt, fixtures parity, unit tests). The remaining 5 are either failing or
unimplemented. Specific issues:

1. **Clippy** — 4 dead_code items in `ark-resolve` need to be either used or
   annotated with `#[allow(dead_code)]`.
2. **verify quick** — Doc examples in `lang-uplift-gap-ledger.md` and `spec.md`
   contain code with stale language syntax; `check-links.sh` has a shell syntax
   error at line 24.
3. **verify component** — 22 renamed-export failures for primitive types; likely
   a WIT binary name-mangling gap.
4. **verify --selfhost-parity** — 3 trait diagnostic regressions.
5. **opt-equiv** — Never implemented; needs a tracking issue if it becomes a
   release gate.

## Required Verification

- Run clippy with warnings as errors
- Check code formatting
- Run verify-harness quick checks
- Run verify-harness component interop
- Run verify-harness fixtures (selfhost parity)
- Run verify-harness selfhost CLI + diag parity
- Run unit tests

## Close Gate

All pre-release CI checks must pass without warnings or errors. The five current
failures block closure.

## Primary Paths

- Clippy configuration (`crates/ark-resolve/src/manifest.rs` dead code)
- Doc examples in `docs/design/lang-uplift-gap-ledger.md` and `docs/language/spec.md`
- Link checker script (`scripts/check/check-links.sh`)
- Component interop WIT bindings for renamed primitive exports
- Selfhost diagnostic parity fixtures

## Non-Goals

- ark-llvm testing (excluded from scope)
- opt-equiv (not implemented; not tracked elsewhere)
