---
Status: done
Created: 2026-06-15
ID: 664
Track: language-design
Parent: 124
Depends on: 652, 653, 663
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v4 exit: no
ADR candidate: yes
Implementation target: "Use Ark (src/compiler/*.ark) per #529 selfhost transition."
Status note: Closed — general WIT record/enum import bindings beyond i32-only fixture tables.
---

# 664 — general record/enum WIT import bindings

## Summary

Extend WIT import registration and typecheck beyond hardcoded i32-only fixture tables.
Parse WIT `record` and `enum` types from interface documents, register struct/enum/variant
symbols in resolver scope, and build function signatures dynamically from parsed WIT types.

## Parent

Umbrella: [#124 WIT component import syntax](../done/124-wit-component-import-syntax.md)

## Acceptance

- [x] WIT parser extracts `record` and `enum` definitions from interface bodies
- [x] Resolver registers WIT record types, enum types, and enum variants under import alias
- [x] Typechecker collects WIT function signatures from parsed documents (not package-id tables)
- [x] Typechecker merges WIT enum defs for match exhaustiveness registration path
- [x] Fixture `tests/fixtures/wit_import/check/record_enum_general.ark` typechecks with `--wit`
- [x] `python3 scripts/manager.py verify quick` exits 0

## Out of scope

- #665 — compose + wasmtime round-trip E2E

## Close gate

`python3 scripts/check/gate-664-wit-import-record-enum-bindings.py`

## References

- `src/compiler/component/wit_parse_text.ark`
- `src/compiler/resolver/wit_register.ark`
- `src/compiler/typechecker/module_wit.ark`
- `tests/fixtures/wit_import/geo_types.wit`
