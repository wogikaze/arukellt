# 586 — Phase 5: Delete `crates/ark-llvm`

**Status**: open
**Created**: 2026-04-23
**Updated**: 2026-04-23
**ID**: 586
**Depends on**: 559
**Track**: selfhost-retirement
**Orchestration class**: implementation-ready
**Orchestration upstream**: #559
**Blocks**: 561
**Blocks v5**: no
**Source**: #529 Phase 5 — T4 native LLVM backend scaffold (last live Rust consumer of `ark_mir` outside `ark-wasm`).

**Implementation target**: Per #529 Phase 5, this issue removes exactly one Rust crate (`crates/ark-llvm`). No Ark product code is added or changed; this is retirement work scoped to a single crate.

## Summary

`crates/ark-llvm` is the T4 (native/LLVM) backend scaffold. Per `docs/current-state.md`, T4 is documented as "scaffold: ark-llvm exists, requires LLVM 18, no test infrastructure" and is excluded from default verification (`cargo test --workspace --exclude ark-llvm`, `.github/workflows/ci.yml` exclusion, `scripts/gate_domain/checks.py:135-137,364-366`).

It is the **last live consumer of `ark_mir` symbols outside `crates/ark-wasm`** (see #561 stop-report 2026-04-23: `crates/ark-llvm/src/emit.rs:7  use ark_mir::mir::*;` + `Cargo.toml:10 ark-mir = { path = "../ark-mir" }`). This blocks #561 from completing even after #562 lands.

T4 has not progressed beyond scaffold and there is no live work track scheduled to advance it before Phase 5 finishes. Deletion is the disposition.

## Pre-deletion invariants (must hold before starting)

Record numeric values; do **not** start the deletion if any item is missing.

- [ ] `python scripts/manager.py selfhost fixpoint` rc=0
- [ ] `python scripts/manager.py selfhost fixture-parity` PASS=<N> FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py selfhost parity --mode --cli` PASS=<N> FAIL=0 (record baseline)
- [ ] `python scripts/manager.py selfhost diag-parity` PASS=<N> FAIL=0 SKIP=<N> (record baseline)
- [ ] `python scripts/manager.py verify` rc=0 (record baseline)
- [ ] No remaining `cargo run -p ark-llvm`-style invocation anywhere reachable from `scripts/` or `.github/workflows/` (`rg "ark-llvm" scripts/ .github/workflows/` should yield only the explicit-exclusion lines, which will be cleaned up)
- [ ] No `use ark_llvm` outside the crate itself (`rg "\bark_llvm\b" crates/ src/` should be empty)

## Acceptance

- [ ] `crates/ark-llvm/` directory removed
- [ ] Workspace root `Cargo.toml` `members` array no longer lists `crates/ark-llvm`
- [ ] Workspace root `Cargo.toml` `[workspace.dependencies]` no longer aliases `ark-llvm`
- [ ] No other crate's `Cargo.toml` lists `ark-llvm`
- [ ] `Cargo.lock` regenerated and committed without `name = "ark-llvm"`
- [ ] CI / verification gate exclusion lines that named `--exclude ark-llvm` are removed (no longer needed):
  - `.github/workflows/ci.yml` (search for `ark-llvm`)
  - `scripts/gate_domain/checks.py` lines around 135-137 and 364-366
  - any `cargo test --workspace --exclude ark-llvm` invocation
- [ ] `docs/current-state.md` T4/native row updated to reflect that the scaffold has been removed (T4 status becomes "not-implemented" or row is dropped, per current-state convention)
- [ ] Re-run all 5 baseline commands; PASS/FAIL counts identical-or-better

## Resolution requirement

When closed, append a `## Resolution` section recording:

- Pre-deletion baselines (5 numbers)
- Post-deletion baselines (5 numbers)
- Commit sha
- Confirmation that `rg "ark_llvm|ark-llvm"` outside `issues/done/` returns zero
- ADR cross-link if any T4 disposition ADR is added

## Notes

- This is a scaffold deletion, not a regression. T4 has no users, no tests, no docs commitments beyond the `current-state.md` "scaffold" row.
- If a future T4 backend is desired, it will be re-built selfhost-native per #529 Phase 7 strategy (no Rust crate revival).
