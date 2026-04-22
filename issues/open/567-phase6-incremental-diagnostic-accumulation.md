# 567 — Phase 6/A3: Selfhost resolver / typechecker — incremental diagnostic accumulation

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 567
**Depends on**: 566
**Track**: selfhost-frontend
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #566
**Blocks**: 568
**Blocks v5**: no
**Source**: #529 Phase 6 — IDE Frontend / LSP / DAP migration

**Implementation target**: Per #529 Phase 6, IDE-side functionality is reimplemented in Ark (`src/`) so that the Rust IDE crates can be retired in Phase 7. This issue covers exactly one concern; do **not** expand scope.

## Summary

Resolver and typechecker must accumulate diagnostics across all top-level items instead of stopping at the first error, so the IDE can surface multiple problems per edit. Existing batch-CLI exit-code and first-error message remain unchanged.

## Acceptance

- [ ] `resolve_program` and `typecheck_program` (or selfhost equivalents) return `(Result, Vec<Diagnostic>)` with all recoverable errors collected
- [ ] CLI exit code and first-error textual diagnostic remain identical for every existing fixture (regression guard)
- [ ] At least one new `tests/fixtures/selfhost/multi_diag_*.ark` fixture proves multiple diagnostics are emitted in one run
- [ ] No selfhost SKIP added
- [ ] 4 canonical gates green with FAIL=0 and SKIP delta = 0

## Required verification (close gate)

Each command MUST be executed; record exit code and (where relevant) PASS/FAIL counts.

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity

```

**REBUILD_BEFORE_VERIFY**: yes

## STOP_IF

- Any of the 4 canonical selfhost gates regresses (FAIL>0 or SKIP delta > 0) → revert and **STOP**.
- The new code path causes a fixture under `tests/fixtures/` to fail → revert and **STOP**.
- This issue's scope cannot be implemented without modifying a forbidden path → open a sibling issue and **STOP**.
- The required upstream behavior is not yet present in selfhost → mark `blocked-by-upstream` and **STOP**.

## False-done prevention checklist (close-gate reviewer must verify all)

1. [ ] Acceptance items each correspond to repo-visible evidence (file path, line, or test name)
2. [ ] Required verification commands are recorded with their exit codes in the close note
3. [ ] 4 canonical gates: numeric Δ recorded; `FAIL=0` and `SKIP_delta=0`
4. [ ] No SKIP added to `scripts/selfhost/checks.py`
5. [ ] No `.selfhost.diag` lenient pattern added without matching real selfhost output (verified by running selfhost on the fixture and grepping for the pattern)
6. [ ] No fixture removed or weakened
7. [ ] commit hash listed; `git show --stat <hash>` shows only PRIMARY / ALLOWED ADJACENT paths
8. [ ] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched
9. [ ] At least one new behavioral test covers the new code path (cite path)

## Primary paths

- `src/compiler/resolver.ark`
- `src/compiler/typechecker.ark`
- `tests/fixtures/selfhost/multi_diag_*.ark`

## Allowed adjacent paths

- `src/compiler/parser.ark` (read-only consumption of #566 output)
- `tests/fixtures/manifest.toml`

## Forbidden paths

- `crates/` (Rust IDE crates remain in place during Phase 6; deletion is Phase 7)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions)
- Other `src/compiler/*.ark` files outside PRIMARY paths
- Any `tests/fixtures/**` deletion

## Commit discipline

- One logical commit per slice. Suggested message: `feat(ide): resolver/typechecker incremental diagnostics (refs #567)`

## Close-note evidence schema (required)

```text
commit: <hash>
acceptance: <each [ ] → [x] with evidence>
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
  diag parity:     PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
new tests added: <paths>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
```
