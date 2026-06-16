---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 672
Track: language-design
Parent: 124
Depends on: "664 (wit-import-record-enum-bindings, done)"
Orchestration class: design-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 3
Source: P1 WIT type binding generation checklist audit 2026-06-17
---

# 672 — WIT type binding code generation

## Summary

Issue #664 registers parsed WIT `record` / `enum` types in resolver scope and typechecks
calls against parsed signatures. This issue tracks **generated Arukellt surface types**
(Option/Result/tuple bindings, nested fields, stable mangling) rather than
fixture-only registration tables.

## Acceptance

- [ ] Generate Arukellt struct bindings from WIT `record` declarations
- [ ] Generate enum bindings from WIT `enum` and variant-style `variant` declarations
- [ ] Generate `Option<T>` / `Result<T,E>` / tuple bindings from WIT equivalents
- [ ] Nested record / option / result fields in generated bindings
- [ ] `list<string>` and `list<record>` record fields
- [ ] Reject recursive WIT types with clear diagnostic
- [ ] Reject unsupported resource handle fields with `E0402` (beyond #473 fixtures)
- [ ] Preserve WIT package/interface metadata on generated bindings
- [ ] Stable name mangling + binding dump (`--emit wit-bindings` or dump phase)
- [ ] Tests: kebab-case field names, reserved keyword fields, enum case normalization,
      variant payload lowering
- [ ] Gate `scripts/check/gate-672-wit-type-binding-codegen.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `src/compiler/component/wit_parse_text.ark`
- `src/compiler/component/wit_names_import.ark`
- `issues/done/664-wit-import-record-enum-bindings.md`
