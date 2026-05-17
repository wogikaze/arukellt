---
Status: done
Created: 2026-05-16
Updated: 2026-05-17
ID: 628
Track: main
Parent: 529
Orchestration class: done
Depends on: 627
Blocks: 571
---

# 529 Phase 6/C: LSP Minimum Viable

## Summary

Phase 6/C of #529: create a minimal Language Server Protocol implementation in selfhost Ark. The LSP server uses the analysis API (Phase 6/B) to provide IDE features: diagnostics on open/change, hover information, and go-to-definition.

Handlers are implemented in order of increasing complexity.

## Acceptance

- [x] LSP exists with JSON-RPC framing entry point (`src/compiler/lsp.ark`; actual path chosen in #569).
- [x] Implements handler: `initialize` (returns server capabilities).
- [x] Implements handler: `textDocument/didOpen` (triggers analysis, publishes diagnostics).
- [x] Implements handler: `textDocument/didChange` (full-sync re-triggers analysis).
- [x] Implements handler: `textDocument/hover` (returns symbol information at position).
- [x] Implements handler: `textDocument/definition` (returns source location of symbol definition).
- [x] Implements handler: `textDocument/publishDiagnostics` (published from server to client after analysis).
- [x] End-to-end tests drive the LSP handlers (`lsp_lifecycle.lsp-script`, `lsp_hover_definition.lsp-script`).
- [x] No SKIP added to `scripts/selfhost/checks.py`.
- [x] 4 canonical selfhost gates green with FAIL=0 and SKIP delta = 0.

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

## Close Note (2026-05-17)

Closed as the aggregate parent for #569 and #570. The original `src/ide/lsp.ark`
path was resolved in #569/#570 to `src/compiler/lsp.ark` so the selfhost module
loader can import it beside `main.ark`.

Evidence:

- #569 implements initialize, didOpen, didChange, and publishDiagnostics.
- #570 implements hover and definition.
- `python3 scripts/check/check-lsp-lifecycle.py`: PASS, 2 scripts.
- Current `python scripts/manager.py verify quick`: PASS, 23/23.
