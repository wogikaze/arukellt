# 494 — Selfhost MIR: SSA formation pass

**Status**: open
**Created**: 2026-04-14
**Updated**: 2026-04-15
**ID**: 494
**Depends on**: 493, 503
**Track**: selfhost
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Source**: audit — issues/done/211-selfhost-mir-lower-fn-bodies.md "Out of scope (deferred)"

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

- [ ] Selfhost MIR pipeline produces SSA-form IR with phi nodes at join points
- [ ] At least one fixture demonstrates SSA phi elimination for a simple branch
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; MIR dump shows phi nodes at join points.
