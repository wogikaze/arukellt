---
Status: done
Created: 2026-07-14
Updated: 2026-07-26
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

- [x] `std::toml` parses full TOML 1.0 (table headers, array-of-tables,
      inline tables, dotted keys, multiline strings, datetime literals)
- [x] `toml_get` / `toml_table_keys` traverse nested tables
- [x] `find_toml_section` / `find_toml_value` are public in `std::toml`
- [x] No compiler-internal TOML implementation files remain outside `std::toml`
      (`script_toml.ark` deleted; `symbol_index_paths.ark` kept for URI/path
      helpers only — TOML wrapper removed)
- [x] Negative fixtures for malformed TOML (unclosed tables, bad arrays)


## Closure note (2026-07-26)

Verdict: **APPROVE**. Implementation: `0e15762b`. Close docs/index: this commit.

Evidence:
- `stdlib_toml` fixtures: 118/118 PASS (wasm32 / wasi-p1)
- `python3 scripts/manager.py verify lane`: PASS
- `script_toml.ark` deleted; callers use `std::toml`
- `find_toml_section` / `find_toml_value` / `toml_get` / `toml_table_keys` public
- Negative fixtures present (`toml_err_*`, updated invalid-* expecteds)
- `symbol_index_paths.ark` retained for URI/path helpers only (no TOML impl)


## Scope

- `std/toml/mod.ark` — full TOML 1.0 parser, section/value helpers
- Compiler `main/script_toml.ark`, `lsp/symbol_index_paths.ark` — delegate
  to `std::toml`, delete local copies
