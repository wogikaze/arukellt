---
Status: done
Created: 2026-04-22
Updated: 2026-04-22
ID: 566
Track: selfhost-frontend
Depends on: 565
Orchestration class: blocked-by-upstream
Orchestration upstream: #565
---

# 566 — Phase 6/A2: Selfhost parser.ark — partial AST recovery
**Blocks**: 568
**Blocks v5**: no
**Source**: #529 Phase 6 — IDE Frontend / LSP / DAP migration

**Implementation target**: Per #529 Phase 6, IDE-side functionality is reimplemented in Ark (`src/`) so that the Rust IDE crates can be retired in Phase 7. This issue covers exactly one concern; do **not** expand scope.

## Summary

Building on #565, the selfhost parser must produce a partial AST (with explicit `Error` / `Missing` nodes) instead of discarding the entire tree on the first syntax error. Batch-CLI output for well-formed input remains unchanged.

## Acceptance

- [x] `parse_program` returns `(MaybePartialAst, Vec<Diagnostic>)` and never aborts the whole tree on a single recoverable syntax error
- [x] Error / Missing AST node variants exist and are emitted with span info
- [x] At least one new `tests/fixtures/selfhost/parser_recovery_*.ark` fixture covers a recovery path with a `.diag` expectation
- [x] Well-formed fixtures still produce byte-identical AST dumps
- [x] 4 canonical gates green with FAIL=0 and SKIP delta = 0

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

1. [x] Acceptance items each correspond to repo-visible evidence (file path, line, or test name)
2. [x] Required verification commands are recorded with their exit codes in the close note
3. [x] 4 canonical gates: numeric Δ recorded; `FAIL=0` and `SKIP_delta=0`
4. [x] No SKIP added to `scripts/selfhost/checks.py`
5. [x] No `.selfhost.diag` lenient pattern added without matching real selfhost output (verified by running selfhost on the fixture and grepping for the pattern)
6. [x] No fixture removed or weakened
7. [x] commit hash listed; `git show --stat <hash>` shows only PRIMARY / ALLOWED ADJACENT paths
8. [x] `python scripts/check/check-docs-consistency.py` rc=0 if docs were touched
9. [x] At least one new behavioral test covers the new code path (cite path)

## Primary paths

- `src/compiler/parser.ark`
- `tests/fixtures/selfhost/parser_recovery_*.ark`

## Allowed adjacent paths

- `src/compiler/lexer.ark` (read-only consumption of #565 output; no behavioral change here)
- `tests/fixtures/manifest.toml`

## Forbidden paths

- `crates/` (Rust IDE crates remain in place during Phase 6; deletion is Phase 7)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions)
- Other `src/compiler/*.ark` files outside PRIMARY paths
- Any `tests/fixtures/**` deletion

## Commit discipline

- One logical commit per slice. Suggested message: `feat(ide): parser.ark partial AST recovery for IDE consumers (refs #566)`

## Close-note evidence schema (required)

```text
commit: <hash>
acceptance: <each checkbox marked with evidence>
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
  diag parity:     PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
new tests added: <paths>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
```

## Close note

```text
commit: <filled-after-merge>
acceptance:
  - parse_program: src/compiler/parser.ark — `pub fn parse_program(tokens) -> ParseProgramResult { decls, errors }`; partial-AST + diagnostics shape; never aborts (sync_to_decl_start recovery)
  - Error/Missing variants: NK_ERROR (96), NK_MISSING (97) constants + AstNode_error / AstNode_missing constructors with Span; rendered "Error" / "Missing" by node_kind_name
  - Recovery strategy: parse_module records (errors, pos) before each parse_decl; on error without forward progress it bumps and synchronises to the next decl-start keyword (fn/pub/struct/enum/trait/impl/use/import)
  - New fixture: tests/fixtures/selfhost/parser_recovery_decls.{ark,diag,selfhost.diag} — three valid `fn` decls separated by ") ) )" garbage; recovery yields exactly 1 parse error and parser still emits all three FnDecl nodes
  - Well-formed inputs unchanged: NK_ERROR/NK_MISSING are only produced on error paths; existing 321 run: fixtures byte-identical (fixture-parity PASS=321)
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=321 FAIL=0 SKIP=41 → PASS=321 FAIL=0 SKIP=41
  diag parity:     PASS=13  FAIL=0 SKIP=22 → PASS=14  FAIL=0 SKIP=22  (+1 new fixture)
  cli parity:      rc=0 → rc=0
new tests added:
  - tests/fixtures/selfhost/parser_recovery_decls.ark
  - tests/fixtures/selfhost/parser_recovery_decls.diag
  - tests/fixtures/selfhost/parser_recovery_decls.selfhost.diag
  - tests/fixtures/manifest.txt (registered new diag: entry)
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
```
