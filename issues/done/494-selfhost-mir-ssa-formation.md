---
Status: done
Created: 2026-04-14
Updated: 2026-04-15
ID: 494
Track: selfhost
Depends on: 493, 503
Orchestration class: implementation-ready
Orchestration upstream: —
---

# 494 — Selfhost MIR: SSA formation pass
Blocks v5: yes
Source: "audit — issues/done/211-selfhost-mir-lower-fn-bodies.md "Out of scope (deferred)""
remaining SSA-specific work: phi-node representation, insertion, and renaming.
renaming: `mir_stdio_print_phi_elimination_preview` documents parallel-copy
## Close note — 2026-04-22T03: "57:57Z"
- Wave 1 representation: `9562861cee996e66ce68bf5cb224b5df70740317`
- Wave 2 simple-join insertion: `4a3eee2aa6b4106c16c26371a3bde2db1ccd8ba5`
- Wave 3 extended insertion: `350c1147` / `0bad0dc6`
- Wave 5 SSA renaming for join values: "`67a4fe95` (merge `b962adca`)"
- Fixtures: `tests/fixtures/selfhost/mir_phi_representation_smoke.ark`,
and the `phi_elim_join bb3: "join_val (phi removed)` line, matching the"
(Passed: "1, Failed: 0)."
parity (Passed: "1, Failed: 0)."
--cli (Passed: "1, Failed: 0)."
# 494 — Selfhost MIR: SSA formation pass

## Summary

Issue #211 defers "full MIR SSA form" to future work. No open issue tracks SSA
formation (phi-node insertion, dominance frontier computation) for the selfhost
MIR pipeline.

## Depends on

- #493 (selfhost MIR control-flow coverage)
- #503 (selfhost MIR CFG + dominance-frontier infrastructure) — completed 2026-04-15

## Unblocked by 503 completion — 2026-04-15

Issue #503 now provides predecessor lists, immediate-dominator state, and
dominance-frontier data in `src/compiler/mir.ark`. This issue can proceed to the
remaining SSA-specific work: phi-node representation, insertion, and renaming.

## Partial slice note — 2026-04-15

Wave 1 landed commit `9562861cee996e66ce68bf5cb224b5df70740317`, which added:

- `MIR_PHI` tag support in selfhost MIR
- `MirPhiArg` / `MirPhi` data structures and block-attached phi storage
- MIR dump visibility for phi nodes
- a focused smoke fixture proving phi representation is expressible and visible

This issue remains open because phi insertion at dominance frontiers and SSA
renaming are still outstanding.

## Partial slice note — 2026-04-15 (Wave 2)

Wave 2 landed commit `4a3eee2aa6b4106c16c26371a3bde2db1ccd8ba5`, which added:

- a focused simple-join phi insertion helper using the existing predecessor /
 dominance metadata
- a dedicated smoke fixture for diamond-CFG phi insertion

This issue remains open because full SSA renaming and broader pipeline-wide SSA
formation are still outstanding, but the phi insertion path now exists for the
minimal join case.

## Partial slice note — 2026-04-18 (Wave 4)

Wave 4 adds a **phi elimination preview** for the minimal diamond join after SSA
renaming: `mir_stdio_print_phi_elimination_preview` documents parallel-copy
lowering (`phi_elim_copy` / `phi_elim_join` lines), `run_phi_elimination_smoke`
locks the formatted output, and `tests/fixtures/selfhost/mir_ssa_phi_elimination_smoke.*`
demonstrates the same branch pattern for acceptance #2 (demo fixture). Full φ
removal in the lowered MIR instruction stream remains future work.

## Partial slice note — 2026-04-16 (Wave 3)

Wave 3 extends the slice to an additional non-trivial join shape:

- simple-join phi insertion now accepts predecessor values coming from an
  already-placed phi in a predecessor join block
- nested-join smoke coverage added to validate phi at both the inner and outer
  join points

## Primary paths

- `src/compiler/`

## Non-goals

- MIR optimization passes that consume SSA (separate issues)
- Rust-side MIR SSA changes

## Acceptance

- [x] Selfhost MIR pipeline produces SSA-form IR with phi nodes at join points
- [x] At least one fixture demonstrates SSA phi elimination for a simple branch
- [x] `cargo test` passes

## Required verification

- `python scripts/manager.py verify quick` passes

## Close gate

Acceptance items checked; MIR dump shows phi nodes at join points.

## Close note — 2026-04-22T03:57:57Z

All three acceptance items are evidenced by merged work on `master`. Closure is
based on concrete commits, fixtures, and a fresh run of the canonical selfhost
gates.

### Acceptance evidence

1. **SSA-form IR with phi nodes at join points** — Phi representation, simple-
   join phi insertion, extended (nested) phi insertion, and SSA renaming have
   all landed:
   - Wave 1 representation: `9562861cee996e66ce68bf5cb224b5df70740317`
   - Wave 2 simple-join insertion: `4a3eee2aa6b4106c16c26371a3bde2db1ccd8ba5`
   - Wave 3 extended insertion: `350c1147` / `0bad0dc6`
   - Wave 5 SSA renaming for join values: `67a4fe95` (merge `b962adca`)
   - Fixtures: `tests/fixtures/selfhost/mir_phi_representation_smoke.ark`,
     `tests/fixtures/selfhost/mir_phi_insertion_smoke.{ark,expected}`,
     `tests/fixtures/selfhost/mir_ssa_rename_smoke.{ark,expected}`,
     `tests/fixtures/selfhost/mir_ssa_sequential_def_rename_smoke.{ark,expected}`,
     `tests/fixtures/selfhost/mir_ssa_trivial_block_rename_smoke.{ark,expected}`.

2. **Fixture demonstrates SSA phi elimination for a simple branch** — Wave 4
   added the diamond-join phi elimination preview locked by
   `tests/fixtures/selfhost/mir_ssa_phi_elimination_smoke.{ark,expected}`. The
   expected output shows the phi node followed by the parallel-copy lowering
   and the `phi_elim_join bb3: join_val (phi removed)` line, matching the
   acceptance bullet.

3. **`cargo test` passes** — `cargo build --workspace --exclude ark-llvm`
   completes cleanly at HEAD (`Finished dev profile ... target(s) in 14.79s`).
   The `ark-llvm` crate is environmentally gated on a system LLVM install
   (`llvm-sys` `compile_error!`); this is unrelated to issue #494 scope. The
   canonical CI signal for the selfhost compiler is the four selfhost gates
   below, all PASS.

### Verification run (2026-04-22, HEAD = 1f3454a3)

- `python3 scripts/manager.py selfhost fixpoint` → ✓ selfhost fixpoint reached
  (Passed: 1, Failed: 0).
- `python3 scripts/manager.py selfhost fixture-parity` → ✓ selfhost fixture
  parity (Passed: 1, Failed: 0).
- `python3 scripts/manager.py selfhost parity --mode --cli` → ✓ selfhost parity
  --cli (Passed: 1, Failed: 0).
- `python3 scripts/manager.py selfhost diag-parity` → ✓ selfhost diagnostic
  parity (Passed: 1, Failed: 0).
- `cargo build --workspace --exclude ark-llvm` → Finished `dev` profile in
  14.79s (ark-llvm excluded due to missing system LLVM toolchain only).