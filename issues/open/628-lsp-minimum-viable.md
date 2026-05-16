---
Status: open
Created: 2026-05-16
Updated: 2026-05-16
ID: 628
Track: main
Parent: 529
Orchestration class: blocked-by-upstream
Depends on: 627
Blocks: 571
---

# 529 Phase 6/C: LSP Minimum Viable

## Summary

Phase 6/C of #529: create a minimal Language Server Protocol implementation in selfhost Ark. The LSP server uses the analysis API (Phase 6/B) to provide IDE features: diagnostics on open/change, hover information, and go-to-definition.

Handlers are implemented in order of increasing complexity.

## Acceptance

- [ ] `src/ide/lsp.ark` exists with a stdio-loop entry point for JSON-RPC
- [ ] Implements handler: `initialize` (returns server capabilities)
- [ ] Implements handler: `textDocument/didOpen` (triggers analysis, publishes diagnostics)
- [ ] Implements handler: `textDocument/didChange` (incremental or full-sync re-trigger analysis)
- [ ] Implements handler: `textDocument/hover` (returns symbol information at position)
- [ ] Implements handler: `textDocument/definition` (returns source location of symbol definition)
- [ ] Implements handler: `textDocument/publishDiagnostics` (published from server to client after analysis)
- [ ] At least one end-to-end test drives the LSP handlers (e.g., send `didOpen`, verify `publishDiagnostics` notification received)
- [ ] No SKIP added to `scripts/selfhost/checks.py`
- [ ] 4 canonical selfhost gates green with FAIL=0 and SKIP delta = 0

## Scope

**In scope:**
- LSP minimum viable handlers in `src/ide/lsp.ark`
- stdio transport for JSON-RPC
- Integration with analysis API from Phase 6/B
- Diagnostics publishing on file open/change
- Hover and go-to-definition support

**Out of scope:**
- Text synchronization beyond `didOpen`/`didChange` (no `didClose`/`didSave` initially)
- Code completion, signature help, formatting, code actions
- Workspace-level features (workspace symbols, folder watch)
- DAP (Phase 6/D, tracked by #571)
- Incremental reparse optimization (tracked separately in #099)

## Primary paths

- `src/ide/lsp.ark` (new file — LSP stdio server)

## Allowed adjacent paths

- `src/ide/api.ark` (analysis API consumer)
- `tests/fixtures/ide/` (LSP test fixtures)
- `tests/fixtures/manifest.toml`

## Upstream / Depends on

- #627 (Phase 6/B: Analysis API) — LSP handlers consume the analysis API
- #626 (Phase 6/A: IDE-Ready Frontend) — indirectly required for error recovery in analysis

## Blocks

- #571 (Phase 6/D: DAP scaffold) — LSP infrastructure is prerequisite for shared IDE patterns

## Required verification (close gate)

```bash
python scripts/manager.py verify
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

## STOP_IF

- Any of the 4 canonical selfhost gates regresses (FAIL>0 or SKIP delta > 0) — revert and STOP
- LSP implementation requires changes to batch compiler paths (`src/compiler/*.ark` beyond PRIMARY paths) that cause regressions — revert and STOP
- The analysis API is not yet sufficient for LSP needs — return to Phase 6/B and STOP
- Scope expands to DAP or workspace-level features — open sibling issues and STOP

## Close gate

Close when all 6 LSP handlers are implemented with at least one end-to-end test, and required verification passes with FAIL=0 and SKIP delta = 0.
