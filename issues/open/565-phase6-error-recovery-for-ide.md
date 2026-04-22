# 565 — Phase 6/A1: Selfhost lexer.ark — error recovery for IDE

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 565
**Depends on**: —
**Track**: selfhost-frontend
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks**: 568
**Blocks v5**: no
**Source**: #529 Phase 6 — IDE Frontend / LSP / DAP migration

**Implementation target**: Per #529 Phase 6, IDE-side functionality is reimplemented in Ark (`src/`) so that the Rust IDE crates can be retired in Phase 7. This issue covers exactly one concern; do **not** expand scope.

## Summary

The IDE needs the selfhost lexer to continue past lexical errors and emit a partial token stream with diagnostic markers, instead of aborting on the first error. This issue adds error-recovery behavior to `src/compiler/lexer.ark` while keeping batch-CLI output bit-identical for inputs that do not contain lexical errors.

## Acceptance

- [ ] `lex_program` (or equivalent entry point) returns both a token vector and a diagnostic vector, with no early abort on recoverable lexical errors
- [ ] At least one new fixture under `tests/fixtures/selfhost/lexer_recovery_*.ark` exercises a recovery path with a `.diag` expectation
- [ ] Existing well-formed fixtures produce byte-identical token streams (regression guard)
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

- `src/compiler/lexer.ark`
- `tests/fixtures/selfhost/lexer_recovery_*.ark`

## Allowed adjacent paths

- `tests/fixtures/manifest.toml` (only to register the new fixture(s))
- `docs/compiler/` (single doc note pointing at the new behavior)

## Forbidden paths

- `crates/` (Rust IDE crates remain in place during Phase 6; deletion is Phase 7)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions)
- Other `src/compiler/*.ark` files outside PRIMARY paths
- Any `tests/fixtures/**` deletion

## Commit discipline

- One logical commit per slice. Suggested message: `feat(ide): lexer.ark error recovery for IDE consumers (refs #565)`

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
