---
Status: open
Created: 2026-07-14
ID: 706
Track: stdlib
Depends on: 606
---

# std::wit Full WIT 1.0 Compliance

## Problem

`std/wit/` currently provides a `#044` subset parser (world, interface, func
with bool/u32/string types only). The compiler's `src/compiler/component/`
directory maintains 12 independent WIT files
(`wit_parse_text`, `wit_parse_text_scan`, `wit_parse_flags`,
`wit_parse_resource`, `wit_parse_types`, `wit_parse_import`, `wit_text`,
`wit_types`, `wit_type_defs`, `wit_decl`, `wit_names`,
`wit_names_import`) that implement a different, broader parser with
record/enum/flags/resource support — none of which is shared with `std::wit`.

## Acceptance criteria

- [ ] `std::wit` parses full WIT 1.0 (package, world, interface, record,
      enum, flags, variant, resource, own/borrow, type aliases, use)
- [ ] `std::wit` provides naming helpers (`kebab_name`, `kebab_to_snake`,
      `pascal_case`) used by both std and compiler
- [ ] `std::wit` provides AST→WIT type lowering (`wit_type_from_ast`)
- [ ] No compiler-internal WIT parser files remain outside `std::wit`
      (compiler may retain AST-view facades like `wit_decl` that are
      compiler-specific)
- [ ] Fixture-backed import parsing uses `std::wit` parser

## Scope

- `std/wit/parser.ark`, `std/wit/types.ark`, `std/wit/world.ark` — extend
  to full WIT 1.0
- New: `std/wit/names.ark` — shared naming helpers
- Compiler `component/wit_parse_*.ark`, `component/wit_names*.ark`,
  `component/wit_types.ark` — delegate to `std::wit`, delete local copies
