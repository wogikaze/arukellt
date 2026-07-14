---
Status: done
Created: 2026-06-15
ID: 653
Track: language-design
Parent: 124
Depends on: 652
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Closed — WIT import resolver, typecheck, and MIR WitCall lowering wired for fixture surfaces.
---

# 653 — WIT import resolver, typecheck, and MIR lowering

## Summary

Wire WIT import declarations through resolver and typechecker into `MirStmt::WitCall` lowering.
Register WIT interface functions and record types in scope; enable typed calls like `md::parse_markdown(s)`.

## Parent

Umbrella: [#124 WIT component import syntax](../done/124-wit-component-import-syntax.md)

## Acceptance

- [x] WIT document registration in resolver (`register_wit_imports` equivalent)
- [x] WIT function calls typecheck with WIT→Arukellt type mapping table
- [x] MIR lower emits `MirStmt::WitCall` for WIT import invocations
- [x] WIT record field access typechecks (basic struct binding)
- [x] Fixture compiles through MIR dump stage
- [x] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/124-wit-component-import-syntax.md`
- `issues/done/652-wit-import-parser-grammar.md`
- `src/compiler/resolver/register_wit.ark`, `src/compiler/typechecker/module_wit.ark`, `src/compiler/mir/lower/body_call_wit.ark`
