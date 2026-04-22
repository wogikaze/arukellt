# 559 — Phase 5 prerequisite: selfhost-first execution path & verify/CI switch

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 559
**Depends on**: —
**Track**: selfhost-retirement
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks**: 560, 561, 562, 563, 564
**Blocks v5**: no
**Source**: #529 Phase 5-1 / 5-2 (Operational Guide)

**Implementation target**: This issue is a **prerequisite** for every Phase 5 deletion. It promotes the selfhost wasm to the canonical execution path used by `scripts/manager.py verify` and `.github/workflows/*.yml`, so that subsequent Phase 5 deletion issues (#560–#564) can remove Rust crates without losing CI coverage.

## Summary

Phase 5 of #529 deletes the Rust core compiler crates. Before any deletion is safe, the canonical execution path must already be selfhost-first: `scripts/manager.py verify` and `.github/workflows/*.yml` must invoke the selfhost wasm (or a promoted wrapper) instead of `cargo run -p arukellt` for compilation. This issue codifies that switch and proves it with all 4 canonical selfhost gates green.

## Acceptance

- [ ] `arukellt` user-facing entry point invokes the selfhost wasm via a documented wrapper (script or shim binary), and that wrapper is referenced from `docs/current-state.md`
- [ ] `python scripts/manager.py verify` no longer requires `cargo run -p arukellt` for compilation in any subcommand reachable from the default `verify` path (`rg "cargo run -p arukellt" scripts/` returns only entries explicitly enumerated as out-of-scope and listed in this issue's close note)
- [ ] `.github/workflows/*.yml` no longer invokes `cargo run -p arukellt` on the hot path (`rg "cargo run -p arukellt" .github/workflows/` returns only entries explicitly enumerated and listed in the close note)
- [ ] All 4 canonical selfhost gates: rc=0, FAIL=0, SKIP delta=0
- [ ] `python scripts/manager.py verify` rc=0 with the new execution path
- [ ] No new SKIP added to `scripts/selfhost/checks.py`
- [ ] `docs/current-state.md` updated to describe the selfhost-first execution path

## Required verification (close gate)

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost parity --mode --cli
python scripts/manager.py selfhost diag-parity
rg -n "cargo run -p arukellt" scripts/ .github/workflows/
python scripts/check/check-docs-consistency.py
```

**REBUILD_BEFORE_VERIFY**: yes

## STOP_IF

- Any of the 4 canonical gates regresses → revert and **STOP**.
- The wrapper changes user-visible CLI exit codes for any covered fixture → revert and **STOP**.
- A `cargo run -p arukellt` invocation cannot be removed from CI without first migrating an unrelated subsystem → carve out a focused migration issue and **STOP**.

## False-done prevention checklist (close-gate reviewer)

1. [ ] `rg -n "cargo run -p arukellt" scripts/ .github/workflows/` output cited; remaining entries justified per-line in the close note
2. [ ] All 4 canonical gates: numeric Δ recorded
3. [ ] `python scripts/manager.py verify` rc=0 (full output excerpt cited)
4. [ ] Wrapper artifact path documented (e.g. `scripts/run/arukellt-selfhost.sh` or `bin/arukellt`)
5. [ ] `docs/current-state.md` diff cited
6. [ ] `python scripts/check/check-docs-consistency.py` rc=0
7. [ ] No SKIP added (`git diff scripts/selfhost/checks.py` shows no `*_SKIP` growth)
8. [ ] commit hash listed; `git show --stat <hash>` shows only PRIMARY / ALLOWED ADJACENT paths

## Primary paths

- `scripts/manager.py` (and `scripts/selfhost/`, `scripts/run/` files it invokes)
- `.github/workflows/*.yml` (only invocation lines)
- A new wrapper artifact (script or shim binary), path to be decided in this issue

## Allowed adjacent paths

- `docs/current-state.md` (single section update)
- `docs/adr/` if a new ADR is required to record the launch-path decision

## Forbidden paths

- `crates/` (no crate deletion in this issue; deletion is #560–#564)
- `src/compiler/*.ark` (no Ark product changes)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions)

## Commit discipline

- One logical commit, or at most one per surface (script vs CI), each tagged `chore(selfhost): {surface} — selfhost-first execution path (refs #559)`.

## Close-note evidence schema (required)

```text
commit(s): <hash[, hash]>
wrapper artifact path: <path>
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
  cli parity:      PASS=<N> FAIL=0       → PASS=<N> FAIL=0
  diag parity:     PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
remaining `cargo run -p arukellt` references: <list, each justified>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓
```
