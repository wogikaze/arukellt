---
Status: open
Created: 2026-05-16
Updated: 2026-05-16
ID: 626
Track: main
Parent: 529
Orchestration class: blocked-by-upstream
Depends on: 625
Blocks: 627
---

# 529 Phase 6/A: IDE-Ready Frontend

## Summary

Phase 6/A of #529: extend the selfhost compiler frontend (`lexer.ark`, `parser.ark`, `resolver.ark`, `typechecker.ark`) with IDE-grade error recovery capabilities. Unlike the batch compiler (which fails fast on the first error), the IDE frontend must continue past syntax errors, preserve partial AST structure, and accumulate diagnostics incrementally.

This is a separate project-level effort from the batch compiler work in Phases 1-5.

## Acceptance

- [ ] `src/compiler/lexer.ark` supports error recovery: returns partial token stream past lexer errors
- [ ] `src/compiler/parser.ark` supports error recovery: continues parsing after syntax errors, returns partial AST with error nodes
- [ ] `src/compiler/resolver.ark` accepts partial AST and resolves what it can, recording diagnostics for unresolvable references
- [ ] `src/compiler/typechecker.ark` checks available partial AST segments and records type errors without panicking
- [ ] No SKIP added to `scripts/selfhost/checks.py`
- [ ] 4 canonical selfhost gates green with FAIL=0 and SKIP delta = 0
- [ ] At least one runner test validates error-recovery behavior (e.g., parse incomplete file and verify partial AST is returned)

## Scope

**In scope:**
- Error recovery in lexer/parser (continue past errors, return partial results)
- Partial AST preservation (error nodes instead of abort)
- Incremental diagnostic accumulation (don't halt on first error)
- Resolver/typechecker tolerance for partial/erroneous AST

**Out of scope:**
- Incremental reparse (tracked separately in #099)
- LSP protocol handlers (Phase 6/C)
- Analysis API extraction (Phase 6/B)
- DAP (Phase 6/D, tracked by #571)

## Primary paths

- `src/compiler/lexer.ark`
- `src/compiler/parser.ark`
- `src/compiler/resolver.ark`
- `src/compiler/typechecker.ark`

## Allowed adjacent paths

- `tests/fixtures/ide/` (new test fixtures for error recovery)
- `tests/fixtures/manifest.toml`

## Upstream / Depends on

- #625 (Phase 4: Dual-Run Period) — dual-run stability is prerequisite before IDE changes

## Blocks

- #627 (Phase 6/B: Analysis API) — requires IDE-ready frontend components
- #571 (Phase 6/D: DAP) — depends on IDE-ready frontend

## Required verification (close gate)

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

## STOP_IF

- Any of the 4 canonical selfhost gates regresses (FAIL>0 or SKIP delta > 0) — revert and STOP
- Error recovery changes cause fixture failures under batch compilation — revert and STOP
- The scope expands to LSP protocol handling (that is Phase 6/C) or DAP (Phase 6/D) — open sibling issues and STOP
- Incremental reparse architecture is needed before error recovery can work — reference #099 and STOP

## Close gate

Close when all acceptance items are met, required verification passes with FAIL=0 and SKIP delta = 0, and at least one test validates error-recovery behavior.
