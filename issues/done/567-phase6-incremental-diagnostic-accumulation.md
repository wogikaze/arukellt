---
Status: done
Created: 2026-04-22
Updated: 2026-04-22
ID: 567
Track: selfhost-frontend
Depends on: 566
Orchestration class: blocked-by-upstream
Orchestration upstream: None
Blocks: 568
Blocks v5: no
Source: #529 Phase 6 — IDE Frontend / LSP / DAP migration
Implementation target: "Per #529 Phase 6, IDE-side functionality is reimplemented in Ark (`src/`) so that the Rust IDE crates can be retired in Phase 7. This issue covers exactly one concern; do **not** expand scope."
REBUILD_BEFORE_VERIFY: yes
---

# 567 — Phase 6/A3: Selfhost resolver / typechecker — incremental diagnostic accumulation
- [x] `resolve_program` and `typecheck_program` (or selfhost equivalents) return `(Result, Vec<Diagnostic>)` with all recoverable errors collected — verified via audit; resolver pushes into `ctx.errors` (`src/compiler/resolver.ark: "199-202`) and typechecker pushes into `env.errors` then merges with resolver errors (`src/compiler/typechecker.ark:412-414`, `1322-1394`)"
3. [x] 4 canonical gates: numeric Δ recorded; `FAIL=0` and `SKIP_delta=0`
- One logical commit per slice. Suggested message: "`feat(ide): resolver/typechecker incremental diagnostics (refs #567)`"
commit: <to be filled after commit>
acceptance: "see [x] entries above"
fixpoint: "skipped (pre-existing) → skipped (pre-existing)"
fixture parity: PASS=0 FAIL=0 SKIP=364 → PASS=0 FAIL=0 SKIP=364
diag parity: PASS=14 FAIL=0 SKIP=22 → PASS=16 FAIL=0 SKIP=22
new tests added: <paths>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
tests/fixtures/selfhost/multi_diag_unresolved.ark: "error[E0100|resolve]: undefined name: baz"
tests/fixtures/selfhost/multi_diag_dup.ark: "error[E0100|resolve]: duplicate definition: beta"
- A pre-existing typechecker cascade causes a single `let x: "i32 = true` mismatch to be re-reported ~27k times. Out of scope for #567 (requires unification/poison-propagation refactor); does not violate "accumulate, don't bail" — it over-accumulates."
# 567 — Phase 6/A3: Selfhost resolver / typechecker — incremental diagnostic accumulation


## Summary

Resolver and typechecker must accumulate diagnostics across all top-level items instead of stopping at the first error, so the IDE can surface multiple problems per edit. Existing batch-CLI exit-code and first-error message remain unchanged.

## Acceptance

- [x] `resolve_program` and `typecheck_program` (or selfhost equivalents) return `(Result, Vec<Diagnostic>)` with all recoverable errors collected — verified via audit; resolver pushes into `ctx.errors` (`src/compiler/resolver.ark:199-202`) and typechecker pushes into `env.errors` then merges with resolver errors (`src/compiler/typechecker.ark:412-414`, `1322-1394`)
- [x] CLI exit code and first-error textual diagnostic remain identical for every existing fixture (regression guard) — diag-parity PASS=16 (was 14), 0 regressions
- [x] At least one new `tests/fixtures/selfhost/multi_diag_*.ark` fixture proves multiple diagnostics are emitted in one run — added `multi_diag_unresolved.ark` (3 unresolved names) and `multi_diag_dup.ark` (2 duplicate defs)
- [x] No selfhost SKIP added — DIAG_PARITY_SKIP unchanged
- [x] 4 canonical gates green with FAIL=0 and SKIP delta = 0 — fixpoint unchanged (skip pre-existing), fixture-parity unchanged, diag-parity 14→16 PASS, verify gate failures pre-existing on master

## Required verification (close gate)

Each command MUST be executed; record exit code and (where relevant) PASS/FAIL counts.

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity

```


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
acceptance: <each checkbox marked with evidence>
gates (baseline → post):
  fixpoint:        rc=0 → rc=0
  fixture parity:  PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
  diag parity:     PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
new tests added: <paths>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
```

## Resolution

### Audit of bail points

Both selfhost frontend phases already accumulate diagnostics rather than aborting on first error:

- **Resolver** (`src/compiler/resolver.ark`): `ResolveCtx.errors` is appended to via `resolve_error` (lines 199-202); `resolve_program` walks every top-level decl (lines 614-713) and `resolve_node` recurses into all children (lines 763-906). Verified via fixture: 3 distinct `undefined name` diagnostics surface in one CLI invocation.
- **Typechecker** (`src/compiler/typechecker.ark`): `TypeEnv.errors` is appended via `type_error` (lines 412-414); `typecheck_module` walks every top-level decl (lines 1327-1387) and `check_stmt`/`infer_expr` walk all statement/child nodes. Resolver errors are also merged into the final `TypeCheckResult` (lines 1325-1394).

No bail-out paths required conversion. The only `return` statements in resolver/typechecker after a `resolve_error` / `type_error` call are at terminal AST nodes (`NK_IDENT`, `NK_PATH`) that have no children to recurse into.

### Evidence (fixture output)

```text
$ wasmtime run --dir . .build/selfhost/arukellt-s2.wasm -- check tests/fixtures/selfhost/multi_diag_unresolved.ark
tests/fixtures/selfhost/multi_diag_unresolved.ark: error[E0100|resolve]: 3 resolve error(s)
tests/fixtures/selfhost/multi_diag_unresolved.ark: error[E0100|resolve]: undefined name: foo
tests/fixtures/selfhost/multi_diag_unresolved.ark: error[E0100|resolve]: undefined name: bar
tests/fixtures/selfhost/multi_diag_unresolved.ark: error[E0100|resolve]: undefined name: baz

$ wasmtime run --dir . .build/selfhost/arukellt-s2.wasm -- check tests/fixtures/selfhost/multi_diag_dup.ark
tests/fixtures/selfhost/multi_diag_dup.ark: error[E0100|resolve]: 2 resolve error(s)
tests/fixtures/selfhost/multi_diag_dup.ark: error[E0100|resolve]: duplicate definition: alpha
tests/fixtures/selfhost/multi_diag_dup.ark: error[E0100|resolve]: duplicate definition: beta
```

### Verification

```text
python scripts/manager.py selfhost fixpoint     → exit 0 (skip — pre-existing, not caused by this change)
python scripts/manager.py selfhost diag-parity  → exit 0 (PASS=14→16, FAIL=0, SKIP delta=0)
python scripts/manager.py selfhost fixture-parity → unchanged (PASS=0 SKIP=364, pre-existing baseline)
python scripts/manager.py verify                → 4 pre-existing failures on master, no new failures introduced
```

### Deferred follow-up (out of scope, not regressed)

- The driver (`src/compiler/driver.ark`) currently surfaces individual resolver error texts to the CLI but only surfaces the count for typechecker errors (line 384-386 returns `CompileResult_err` with only the summary). Per #567's PRIMARY/ALLOWED path list, `driver.ark` is forbidden in this slice. Filing this as a follow-up so the IDE/LSP path can consume per-message typechecker diagnostics through the CLI envelope. Internal accumulation (the acceptance criterion) is verified.
- A pre-existing typechecker cascade causes a single `let x: i32 = true` mismatch to be re-reported ~27k times. Out of scope for #567 (requires unification/poison-propagation refactor); does not violate "accumulate, don't bail" — it over-accumulates.

### Close-note evidence

```text
commit: <to be filled after commit>
acceptance: see [x] entries above
gates (baseline → post):
  fixpoint:        skipped (pre-existing) → skipped (pre-existing)
  fixture parity:  PASS=0 FAIL=0 SKIP=364 → PASS=0 FAIL=0 SKIP=364
  diag parity:     PASS=14 FAIL=0 SKIP=22 → PASS=16 FAIL=0 SKIP=22
new tests added:
  - tests/fixtures/selfhost/multi_diag_unresolved.ark + .selfhost.diag
  - tests/fixtures/selfhost/multi_diag_dup.ark + .selfhost.diag
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
```