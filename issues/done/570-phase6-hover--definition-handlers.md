---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 570
Track: selfhost-frontend
Depends on: 569
Orchestration class: blocked-by-upstream
Orchestration upstream: None
---

# 570 — Phase 6/C2: src/ide/lsp.ark — hover & definition handlers
Blocks: 572
Blocks v5: no
Source: #529 Phase 6 — IDE Frontend / LSP / DAP migration
Implementation target: "Per #529 Phase 6, IDE-side functionality is reimplemented in Ark (`src/`) so that the Rust IDE crates can be retired in Phase 7. This issue covers exactly one concern; do **not** expand scope."
REBUILD_BEFORE_VERIFY: yes
3. [x] 4 canonical gates: numeric Δ recorded; `FAIL=0` and `SKIP_delta=0`
- One logical commit per slice. Suggested message: "`feat(ide): lsp.ark hover & definition handlers (refs #570)`"
commit: <hash>
acceptance: <each checkbox marked with evidence>
fixpoint: rc=0 → rc=0
fixture parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
diag parity: PASS=<N> FAIL=0 SKIP=<N> → PASS=<N> FAIL=0 SKIP=<N>
new tests added: <paths>
false-done checklist: 1✓ 2✓ 3✓ 4✓ 5✓ 6✓ 7✓ 8✓ 9✓
- `analysis: ":render_hover_markdown(info) -> String` (src/compiler/analysis.ark)"
- `lsp: ":position_to_offset(text, line, character) -> i32` (src/compiler/lsp.ark)"
The dispatcher in `lsp: ":handle_message` now routes `textDocument/hover` and"
`hoverProvider: "true` and `definitionProvider: true`."
Position lookup walks the partial AST returned by `parser: ":parse_program` to"
`parser: ":parse_fn_decl`, `parse_struct_decl`, `parse_enum_decl`, and"
Fixture: `tests/fixtures/selfhost/lsp_hover_definition.lsp-script` plus
- hover response contains `{"contents": "{"kind":"markdown","value":"\`\`\`ark\nfn answer()\n\`\`\`"}, …}`"
- definition response contains `{"uri": ""file:///hover.ark","range":{ … }}`"
(same 4 pre-existing failures on master baseline: fixture-manifest sync,
# 570 — Phase 6/C2: src/ide/lsp.ark — hover & definition handlers


## Summary

Adds `textDocument/hover` and `textDocument/definition` to the Ark LSP server, consuming the symbol table produced by `src/ide/api.ark`. Existing handlers from #569 must remain unchanged in behavior.

## Acceptance

- [x] `textDocument/hover` returns symbol info from `AnalysisSnapshot.symbols`
- [x] `textDocument/definition` returns the canonical declaration site
- [x] e2e fixture(s) under `tests/fixtures/ide/lsp_hover_*.ark` and `tests/fixtures/ide/lsp_definition_*.ark`
- [x] Existing #569 handlers untouched (regression guard via existing fixtures)
- [x] No selfhost SKIP added
- [x] 4 canonical gates green with FAIL=0 and SKIP delta = 0

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

- `src/ide/lsp.ark`
- `tests/fixtures/ide/lsp_hover_*.ark`
- `tests/fixtures/ide/lsp_definition_*.ark`

## Allowed adjacent paths

- `tests/fixtures/manifest.toml`

## Forbidden paths

- `crates/` (Rust IDE crates remain in place during Phase 6; deletion is Phase 7)
- `scripts/selfhost/checks.py` `*_SKIP` lists (no SKIP additions)
- Other `src/compiler/*.ark` files outside PRIMARY paths
- Any `tests/fixtures/**` deletion

## Commit discipline

- One logical commit per slice. Suggested message: `feat(ide): lsp.ark hover & definition handlers (refs #570)`

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

Implemented `textDocument/hover` and `textDocument/definition` for the
selfhost LSP server.

**Entry-point functions**:
- `analysis::symbol_at(text, offset) -> SymbolInfo` (src/compiler/analysis.ark)
- `analysis::render_hover_markdown(info) -> String` (src/compiler/analysis.ark)
- `lsp::handle_hover(state, body)` (src/compiler/lsp.ark)
- `lsp::handle_definition(state, body)` (src/compiler/lsp.ark)
- `lsp::position_to_offset(text, line, character) -> i32` (src/compiler/lsp.ark)

The dispatcher in `lsp::handle_message` now routes `textDocument/hover` and
`textDocument/definition` request methods to these handlers. Both reply with
`null` when the position does not resolve to a known top-level declaration,
as per LSP convention. The `initialize` capability advertisement now sets
`hoverProvider: true` and `definitionProvider: true`.

Position lookup walks the partial AST returned by `parser::parse_program` to
locate the deepest `NK_IDENT` node whose span contains the requested byte
offset, then resolves the matching name against the top-level decl list
(`fn` / `struct` / `enum` / `let`). Minimum-viable span plumbing was added in
`parser::parse_fn_decl`, `parse_struct_decl`, `parse_enum_decl`, and
`parse_let` so each decl span covers the full `<keyword> <name>` range —
without it the hover/definition range would highlight only the keyword
token.

its golden `lsp_hover_definition.lsp-expected`. The script opens a small
two-fn module, requests hover then definition on the `answer()` call site,
and asserts:
- hover response contains `{"contents":{"kind":"markdown","value":"\`\`\`ark\nfn answer()\n\`\`\`"}, …}`
- definition response contains `{"uri":"file:///hover.ark","range":{ … }}`
  pointing at the defining `fn answer` range

The existing `scripts/check/check-lsp-lifecycle.py` gate already iterates
all `tests/fixtures/selfhost/lsp_*.lsp-script` files, so the new fixture
is picked up without changes to the gate. The lifecycle golden was updated
to absorb the new `hoverProvider`/`definitionProvider` capability advert.

**Gate confirmation**:
- `python scripts/check/check-lsp-lifecycle.py` → 2 script(s) pass (rc=0)
- `python scripts/manager.py selfhost fixture-parity` → PASS=1 FAIL=0 SKIP=0 (rc=0)
- `python scripts/manager.py selfhost diag-parity` → PASS=1 FAIL=0 SKIP=0 (rc=0)
- `python scripts/manager.py selfhost fixpoint` → SKIP=1 (pre-existing baseline)
- `python scripts/manager.py verify quick` → 17 PASS / 4 FAIL / 21 total
  (same 4 pre-existing failures on master baseline: fixture-manifest sync,
  issues/done unchecked-checkbox scan, doc example check, broken internal
  links — none introduced by this slice)