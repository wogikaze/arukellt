---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 669
Track: lsp-navigation
Parent: 124
Depends on: "652 (wit-import-parser-grammar, done)"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: P0 WIT import syntax checklist audit 2026-06-17 — IDE/formatter gaps
---

# 669 — WIT import IDE and formatter surface

## Summary

Parser grammar for `import "namespace:pkg/iface" as alias` landed in #652, but the
formatter, LSP symbol index, completion, hover, go-to-definition, and code actions
do not treat `IMPORT_KIND_WIT` imports. Users get no IDE help for WIT imports and
format-on-save may not preserve WIT import layout.

## Acceptance

- [ ] Formatter recognizes and formats WIT import declarations (`import "…" as …`)
- [ ] LSP parser / symbol index records WIT import aliases in module scope
- [ ] Completion offers WIT import alias names where applicable
- [ ] Hover shows WIT imported function signatures resolved from `--wit` / `ark.toml`
- [ ] Go-to-definition on WIT alias jumps to the loaded WIT interface document
- [ ] Code action suggests missing `as alias` or `--wit` path when WIT call fails
- [ ] Diagnostic when WIT import syntax is used on non-`wasm32-wasi-p2` / non-component
      targets (stable code, e.g. `E05xx`)
- [ ] Golden tests: formatter round-trip for `tests/fixtures/wit_import/parse/*`
- [ ] LSP lifecycle or dedicated gate under `scripts/check/`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Primary paths

- `src/compiler/parser/imports_wit.ark`
- `src/compiler/lsp/` (symbol index, completion, hover)
- `src/compiler/formatter/` (or equivalent format pipeline)
- `tests/fixtures/wit_import/`

## References

- `issues/done/652-wit-import-parser-grammar.md`
- `issues/done/124-wit-component-import-syntax.md`
- `docs/adr/ADR-031-import-syntax-wit-unification.md`
