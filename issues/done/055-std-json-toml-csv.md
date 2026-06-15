---
Status: done
Created: 2026-03-28
Updated: 2026-06-15
ID: 45
Track: stdlib
Depends on: 039, 042, 044
Blocks v3 exit: "no (Experimental — json Stable candidate)"
Status note: Closed 2026-06-15 — recursive JsonValue/TomlValue enums, csv_parse/stringify, 15+ fixtures.
---

# std::json + std::toml + std::csv

## Acceptance

- [x] `JsonValue` recursive enum + parse/stringify/pretty/get helpers
- [x] `TomlValue` recursive enum + toml_parse/toml_stringify
- [x] `csv_parse` / `csv_stringify` / `csv_parse_with_header` (RFC 4180 row parser)
- [x] Fixtures: `stdlib_json/*`, `stdlib_toml/toml_basic.ark`, `stdlib_csv/*` (15+ registered)

## References

- `std/json/mod.ark`, `std/toml/mod.ark`, `std/csv/mod.ark`
