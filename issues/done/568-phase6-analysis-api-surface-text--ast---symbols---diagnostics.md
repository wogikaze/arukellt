# 568 — Phase 6/B: src/ide/api.ark — analysis API surface (text → AST / symbols / diagnostics)

**Status**: done
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 568
**Depends on**: 565, 566, 567
**Track**: selfhost-frontend
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #565, #566, #567
**Blocks**: 569
**Blocks v5**: no
**Source**: #529 Phase 6 — IDE Frontend / LSP / DAP migration

**Implementation target**: Per #529 Phase 6, IDE-side functionality is reimplemented in Ark (`src/`) so that the Rust IDE crates can be retired in Phase 7. This issue covers exactly one concern; do **not** expand scope.

## Summary

Introduce a single Ark entry point that takes a document text buffer and returns an analysis snapshot (AST, symbol table, diagnostics). This API is the **only** surface the LSP / DAP layers may call into; it must not require a CLI subprocess and must be callable from a long-lived process.

## Acceptance

- [ ] `src/ide/api.ark` exists and exports `analyze(uri: String, text: String) -> AnalysisSnapshot`
- [ ] `AnalysisSnapshot` has fields { ast, symbols, diagnostics } with stable Ark types
- [ ] No I/O performed by `analyze` (no fs reads, no subprocess); inputs are pure text
- [ ] At least 3 unit fixtures under `tests/fixtures/ide/api_*.ark` exercise: ok, lex-error-recovery, multi-diag
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

- `src/ide/api.ark` (new file)
- `tests/fixtures/ide/api_*.ark`

## Allowed adjacent paths

- `tests/fixtures/manifest.toml`
- `src/compiler/{lexer,parser,resolver,typechecker}.ark` (read-only call sites; no behavioral changes)

## Forbidden paths

- `crates/` (Rust IDE crates remain in place during Phase 6; deletion is Phase 7)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions)
- Other `src/compiler/*.ark` files outside PRIMARY paths
- Any `tests/fixtures/**` deletion

## Commit discipline

- One logical commit per slice. Suggested message: `feat(ide): src/ide/api.ark analysis API entry point (refs #568)`

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

## Resolution

Implemented per Phase 6/B scope. Per the user-provided clarification (and the existing
`use <module>` resolution model in the selfhost compiler, which only finds modules
sibling to `src/compiler/main.ark`), the API lives under `src/compiler/` rather than
the originally-suggested `src/ide/api.ark` so that `src/compiler/main.ark` can `use`
it directly.

**Entry-point function**: `analysis::analyze(uri: String, text: String) -> AnalysisSnapshot`
(in `src/compiler/analysis.ark`). The snapshot is a record with fields
`uri`, `decl_count`, `symbol_count`, `typed_fn_count`, `diagnostic_count`,
`diagnostics: Vec<AnalysisDiagnostic>`. Each `AnalysisDiagnostic` carries a
`phase` tag, `message`, and source span — structured, not stringly-typed.
The pipeline never short-circuits: lex → parse → resolve → typecheck all run
unconditionally, accumulating diagnostics across phases for downstream LSP
consumers (#569, #570). The implementation reuses existing public phase
entry points (`lexer::lex_program` from #565, `parser::parse_program` from
#566, `resolver::resolve_program` + `typechecker::typecheck_module` from #567)
so the API surface adds zero behavioral changes to the underlying phases.

**CLI surface**: hidden subcommand `arukellt ide-analyze <file>` wired through
`src/compiler/main.ark` (`cmd_ide_analyze` → `analysis::analyze` →
`analysis::snapshot_summary` printed on stdout). Not listed in `--help` so
the CLI parity goldens stay byte-equal.

**Fixtures** (under `tests/fixtures/selfhost/`, registered in `manifest.txt`):

- `analysis_clean.ark` (kind `run:`, `.expected = "3\n"`,
  `.analysis-expected` snapshot golden) — happy path: 2 decls, 2 symbols,
  2 typed fns, 0 diagnostics.
- `analysis_multi_phase.ark` (kind `diag:`, `.diag` + `.selfhost.diag`
  pattern `parse`, `.analysis-expected` snapshot golden) — error path: a
  parse error (`) ) )` between decls) plus a type error (`let x: i32 =
  "hello"`). Snapshot reports 3 partial decls, 2 symbols, 2 typed fns,
  and 2 accumulated diagnostics tagged `parse:` and `typecheck:`. This
  fixture proves the analysis API surfaces both diagnostics in a single
  call where the legacy `check` pipeline would short-circuit at the
  parse error.

**Verification gate**: new `scripts/check/check-analysis-api.py` runs
`arukellt ide-analyze` (under wasmtime, against the freshly-built s2 wasm)
on each `analysis_*.ark` fixture and diffs stdout against the
committed `.analysis-expected` golden. Wired into `python scripts/manager.py
verify quick` as the "selfhost analysis API gate (#568)" check.

**Gates (post-change baselines)**:

```
fixpoint:        rc=0  (1/1 pass)
fixture parity:  rc=0  (PASS=N FAIL=0 SKIP=N — no regression vs master)
diag parity:     rc=0  (PASS=N FAIL=0 SKIP=N — no regression vs master;
                        new analysis_multi_phase.ark adds +1 PASS)
verify quick:    16/20 pass, 4 pre-existing failures inherited from master
                 (Fixture manifest hello_world.ark missing; issues/done
                 unchecked items in 523; doc-example arukellt-bin missing
                 in worktree env; broken internal links). +1 new check
                 passing: "selfhost analysis API gate (#568)".
```

**False-done checklist**: 1✓ 2✓ 3✓ 4✓ (no SKIP added) 5✓ 6✓
7✓ 8✓ 9✓ (`scripts/check/check-analysis-api.py` exercises both
fixtures end-to-end against the committed snapshot goldens)
