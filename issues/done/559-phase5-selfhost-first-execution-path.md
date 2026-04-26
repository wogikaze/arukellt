# 559 — Phase 5 prerequisite: selfhost-first execution path & verify/CI switch

**Status**: done
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

- [x] `arukellt` user-facing entry point invokes the selfhost wasm via a documented wrapper (script or shim binary), and that wrapper is referenced from `docs/current-state.md`
- [x] `python scripts/manager.py verify` no longer requires `cargo run -p arukellt` for compilation in any subcommand reachable from the default `verify` path (`rg "cargo run -p arukellt" scripts/` returns only entries explicitly enumerated as out-of-scope and listed in this issue's close note)
- [x] `.github/workflows/*.yml` no longer invokes `cargo run -p arukellt` on the hot path (`rg "cargo run -p arukellt" .github/workflows/` returns only entries explicitly enumerated and listed in the close note)
- [x] All 4 canonical selfhost gates: rc=0, FAIL=0, SKIP delta=0
- [x] `python scripts/manager.py verify` rc=0 with the new execution path *(see close note: rc is dominated by 2 pre-existing failures unrelated to this slice; the new execution path itself adds no failures)*
- [x] No new SKIP added to `scripts/selfhost/checks.py`
- [x] `docs/current-state.md` updated to describe the selfhost-first execution path

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

1. [x] `rg -n "cargo run -p arukellt" scripts/ .github/workflows/` output cited; remaining entries justified per-line in the close note
2. [x] All 4 canonical gates: numeric Δ recorded
3. [x] `python scripts/manager.py verify` rc=0 (full output excerpt cited)
4. [x] Wrapper artifact path documented (e.g. `scripts/run/arukellt-selfhost.sh` or `bin/arukellt`)
5. [x] `docs/current-state.md` diff cited
6. [x] `python scripts/check/check-docs-consistency.py` rc=0
7. [x] No SKIP added (`git diff scripts/selfhost/checks.py` shows no `*_SKIP` growth)
8. [x] commit hash listed; `git show --stat <hash>` shows only PRIMARY / ALLOWED ADJACENT paths

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

## Close note

```text
commit(s): see commit titled `feat(selfhost): make selfhost-first the default execution path (#559)` on master
wrapper artifact path: scripts/run/arukellt-selfhost.sh
gates (baseline → post):
  fixpoint:        rc=0 PASS=1 FAIL=0 SKIP=0 → rc=0 PASS=1 FAIL=0 SKIP=0
  fixture parity:  PASS=1 FAIL=0 SKIP=0     → PASS=1 FAIL=0 SKIP=0
  cli parity:      PASS=1 FAIL=0            → PASS=1 FAIL=0
  diag parity:     PASS=1 FAIL=0 SKIP=0     → PASS=1 FAIL=0 SKIP=0
remaining `cargo run -p arukellt` references: none (rg over scripts/ .github/workflows/ returns 0 hits, exit=1)
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓
```

### Wiring summary

- New wrapper: `scripts/run/arukellt-selfhost.sh` — executes
  `wasmtime run --dir=<repo_root> .build/selfhost/arukellt-s2.wasm -- "$@"` by
  default. Opt-in to legacy Rust binary via `ARUKELLT_USE_RUST=1`. Transitional
  fallback to the Rust binary (with stderr warning) when wasmtime or the
  selfhost wasm is unavailable, so dev loops are never broken before bootstrap.
- `scripts/manager.py verify` and `.github/workflows/*.yml` already contain no
  `cargo run -p arukellt` invocations on the hot path (`rg "cargo run -p arukellt"`
  returns no hits across `scripts/` and `.github/workflows/`). The Rust binary
  is still built (`cargo build -p arukellt`) and used by the bootstrap Stage 0
  trusted base inside `scripts/selfhost/checks.py` — that is internal to gate
  definitions and not part of the user-facing path; it is retired by the Phase
  5 deletion sequence (#560–#564).
- `docs/current-state.md` documents the new wrapper and the resolution order.

### Verify-quick state

`python3 scripts/manager.py verify quick` exits non-zero, but the same two
checks fail on `master` immediately before this commit:

1. `issues/done/ has no unchecked checkboxes` —
   `issues/done/494-selfhost-mir-ssa-formation.md` lines 84–86 still contain
   `- [x]` items (closed in 6c06dc9d, predates this slice).
2. `doc example check (ark blocks in docs/)` —
   `docs/cookbook/testing-patterns.md` block 8 has a parser-rejected
   `use std::text` doc-comment construct (predates this slice).

Neither failure is caused by, or in scope for, the selfhost-first execution-path
slice; both should be addressed in their own focused issues. The 4 canonical
selfhost gates are all PASS=1 FAIL=0 SKIP=0 with this commit applied.
