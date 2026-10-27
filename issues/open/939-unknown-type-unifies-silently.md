---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 939
Track: compiler
Depends on: "#934, #936"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934, #936"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-040, #933, #935, #938"
---

# 939 — `TY_UNKNOWN` unifies with every type and may survive checking

## Summary

`unify` returns the other side whenever either operand is `TY_UNKNOWN`, with
no diagnostic. `TY_UNKNOWN` is also the default result for unrecognized
builtin calls, arity errors, and several inference paths. It therefore acts
as an unchecked dynamic type: it silences mismatches in the checker and
reaches lowering, where the backend has to guess (#933).

## Evidence (2026-10-27)

- `src/compiler/typechecker/unify_core.ark` L9–17: early return on
  `TY_UNKNOWN` for either operand.
- `src/compiler/typechecker/builtins.ark` `builtin_call_return_type` falls
  back to `TY_UNKNOWN`; `returns.ark` L12–14 skips the return check when
  either side is unknown; `infer_leaf.ark` L40–52 returns `TY_UNKNOWN` for
  unmatched leaves.
- `rg -c TY_UNKNOWN src/compiler/typechecker` ≈ 137 sites.
- `docs/current-state.md` (match exhaustiveness section) already records that
  values whose type "stays unresolved at check time … remain unchecked".
- The `len(42)` repro in #936 type-checks through this path.

## Expected

`TY_UNKNOWN` is only an error-recovery marker after a diagnostic has been
emitted. A body that finishes checking with an unresolved type and no prior
error is rejected (diagnostic or ICE), and lowering never receives an
unknown type.

## Acceptance

- [ ] Unifying with `TY_UNKNOWN` without a prior error in the same body is
      reported (diagnostic or ICE), not silently accepted.
- [ ] After checking a function, any remaining unresolved type variable or
      `TY_UNKNOWN` without a recorded error is an error.
- [ ] `builtin_call_return_type`'s unknown fallback is removed with #936.
- [ ] Regression fixtures: `len(42)` (#936) and an unresolved let-generalized
      scrutinee case are diagnosed on both targets.
- [ ] `python3 scripts/manager.py selfhost fixpoint` stays green.
