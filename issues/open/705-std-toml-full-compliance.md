---
Status: open
Created: 2026-07-14
ID: 705
Track: stdlib
Depends on: 606
---

# std::toml Full TOML 1.0 Compliance

## Problem

`std/toml/mod.ark` currently supports only a bounded subset: blank lines,
comment lines, and simple `key = value` entries. Table headers, arrays of
tables, inline tables, multiline strings, and other TOML 1.0 grammar are
rejected. The compiler's `main/script_toml.ark` and
`lsp/symbol_index_paths.ark` maintain independent `find_toml_section` /
`find_toml_value` helpers that bypass `std::toml` entirely.

## Acceptance criteria

- [ ] `std::toml` parses full TOML 1.0 (table headers, array-of-tables,
      inline tables, dotted keys, multiline strings, datetime literals)
- [ ] `toml_get` / `toml_table_keys` traverse nested tables
- [ ] `find_toml_section` / `find_toml_value` are public in `std::toml`
- [ ] No compiler-internal TOML implementation files remain outside `std::toml`
- [ ] Negative fixtures for malformed TOML (unclosed tables, bad arrays)

## Scope

- `std/toml/mod.ark` — full TOML 1.0 parser, section/value helpers
- Compiler `main/script_toml.ark`, `lsp/symbol_index_paths.ark` — delegate
  to `std::toml`, delete local copies
