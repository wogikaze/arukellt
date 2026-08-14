---
Status: done
Created: 2026-07-14
Updated: 2026-08-14
Closed: 2026-08-14
ID: 706
Track: stdlib
Depends on: 606
---

# 706 — std::wit Full WIT 1.0 Compliance

## Close summary

`std::wit::ast` owns the full WIT parser surface for package, world, interface, record, enum, flags, variant, resource, own/borrow, type aliases, use and functions. `std::wit::parser::parse_full` now exposes that parser as the canonical parser entry point while retaining the legacy World-shaped `parse_wit` compatibility API. `std::wit::types::wit_type_from_ast` owns AST→WIT type rendering and `std::wit::names` owns shared name transformations.

Compiler naming/scanning facades delegate to `std::wit`; fixture-backed WIT import parsing passes source through `std::wit::parser::parse_full`. The duplicate compiler `wit_parse_types.ark` parsed-type model was removed and MIR binding collection now uses the single compiler metadata view backed by the shared std parser.

## Acceptance closure

- [x] Full WIT 1.0 syntax parser exists in `std::wit` and is exposed through `parse_full`.
- [x] Shared kebab/snake/Pascal helpers and AST→WIT lowering live in `std::wit`.
- [x] Compiler WIT naming/scanning are shared facades rather than independent primitives.
- [x] Duplicate compiler parsed-interface type model is removed.
- [x] Fixture-backed compiler imports pass through the full std parser.
- [x] Dedicated consolidation close gate is present.

## Verification

- `scripts/check/gate-706-std-wit-full-compliance.py`
- `scripts/check/gate-component-wit-productization.py`
