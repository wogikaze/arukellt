---
Status: open
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
Status note: Child of #124 — parser/import grammar slice.
---

# 652 — WIT import parser grammar (`import "..." as alias`)

## Summary

Add `import "namespace:pkg/interface" as alias` source syntax to the selfhost parser and AST.
Distinguish WIT imports (string literal) from local imports (identifier) per #124 design option (a).

## Parent

Umbrella: [#124 WIT component import syntax](124-wit-component-import-syntax.md)

## Acceptance

- [ ] Parser accepts `import "namespace:pkg/iface" as alias` and optional alias-less form
- [ ] AST distinguishes `ImportKind::Wit` from local/stdlib imports
- [ ] `--wit` CLI flag loads WIT files (minimal, no ark.toml)
- [ ] Compile-error diagnostics for malformed import strings
- [ ] Parser fixture under `tests/fixtures/wit_import/` compiles through parse stage
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/124-wit-component-import-syntax.md`
- `docs/adr/ADR-026-import-vs-wit-package-syntax.md`
- `src/compiler/parser.ark`, `src/compiler/parser_kinds.ark`
