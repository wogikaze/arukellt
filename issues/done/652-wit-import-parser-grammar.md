---
Status: done
Created: 2026-06-15
ID: 652
Track: language-design
Parent: 124
Depends on: "#074"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
ADR candidate: yes
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Closed — WIT import parser grammar (#652); resolver/MIR in #653.
---

# 652 — WIT import parser grammar (`import "..." as alias`)

## Summary

Add `import "namespace:pkg/interface" as alias` source syntax to the selfhost parser and AST.
Distinguish WIT imports (string literal) from local imports (identifier) per #124 design option (a).

## Parent

Umbrella: [#124 WIT component import syntax](../open/124-wit-component-import-syntax.md)

## Acceptance

- [x] Parser accepts `import "namespace:pkg/iface" as alias` and optional alias-less form
- [x] AST distinguishes `ImportKind::Wit` from local/stdlib imports
- [x] `--wit` CLI flag loads WIT files (minimal, no ark.toml)
- [x] Compile-error diagnostics for malformed import strings
- [x] Parser fixture under `tests/fixtures/wit_import/` compiles through parse stage
- [x] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/124-wit-component-import-syntax.md`
- `docs/adr/ADR-026-import-vs-wit-package-syntax.md`
- `src/compiler/parser/imports_import.ark`, `src/compiler/parser/kinds.ark`

## Close notes (2026-06-15)

- Parser dispatches `import "…"` (WIT, `IMPORT_KIND_WIT` in `AstNode.int_val`) vs `import ident` (local, `IMPORT_KIND_LOCAL`).
- Optional `as alias`; alias-less WIT imports default to the last `/interface` segment (version suffix stripped).
- Malformed WIT package ids emit parse diagnostics (`E0001`).
- Fixtures: `tests/fixtures/wit_import/parse/*`; gate: `scripts/check/gate-652-wit-import-parser.py`.
