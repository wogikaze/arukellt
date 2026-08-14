---
Status: done
Created: 2026-06-17
Updated: 2026-08-14
Closed: 2026-08-14
ID: 672
Track: language-design
Parent: 124
Depends on: 664
Priority: 3
---

# 672 — WIT type binding code generation

## Close summary

`src/compiler/component/wit_bindings.ark` provides a WIT→Arukellt surface renderer for records, enums and payload variants. Nested list/option/result/tuple fields are rendered recursively, kebab-case/reserved-style names pass through shared stable naming helpers, and package/interface metadata is retained in generated output.

Parsed variants now retain case payload WIT types and are registered in resolver scope. Direct recursive value records and resource-handle fields that cannot be represented as ordinary value bindings are rejected with E0402 instead of generating invalid Ark types.

## Acceptance closure

- [x] Record/enum/variant binding generation exists.
- [x] Option/Result/tuple/list and nested fields are rendered recursively.
- [x] Package/interface metadata and stable mangling are preserved.
- [x] Recursive value bindings/resource-handle value fields have explicit diagnostics.
- [x] Nested binding fixture and close gate are present.

## Verification

- `tests/fixtures/wit_import/bindings/nested.wit`
- `scripts/check/gate-672-wit-type-binding-codegen.py`
- `scripts/check/gate-component-wit-productization.py`
