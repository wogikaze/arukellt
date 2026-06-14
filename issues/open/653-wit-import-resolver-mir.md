---
Status: open
Created: 2026-06-15
ID: 653
Track: language-design
Parent: 124
Depends on: 652, 074
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Child of #124 — resolver, typecheck, and MIR lowering slice.
---

# 653 — WIT import resolver, typecheck, and MIR lowering

## Summary

Wire WIT import declarations through resolver and typechecker into `MirStmt::WitCall` lowering.
Register WIT interface functions and record types in scope; enable typed calls like `md::parse_markdown(s)`.

## Parent

Umbrella: [#124 WIT component import syntax](124-wit-component-import-syntax.md)

## Acceptance

- [ ] WIT document registration in resolver (`register_wit_imports` equivalent)
- [ ] WIT function calls typecheck with WIT→Arukellt type mapping table
- [ ] MIR lower emits `MirStmt::WitCall` for WIT import invocations
- [ ] WIT record field access typechecks (basic struct binding)
- [ ] Fixture compiles through MIR dump stage
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/124-wit-component-import-syntax.md`
- `issues/open/652-wit-import-parser-grammar.md`
- `src/compiler/resolver.ark`, `src/compiler/typechecker.ark`, `src/compiler/mir_lower.ark`
