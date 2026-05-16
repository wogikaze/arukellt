---
Status: done
Created: 2026-04-22
Updated: 2026-05-16
ID: 606
Track: stdlib
Orchestration class: implementation-ready
Depends on: 604
Parent: #590
---

# Stdlib Baseline: Structured Data and Semantics Baseline

Closure note (2026-05-16):

All acceptance criteria satisfied:

1. **JSON parse rejects trailing garbage in a negative fixture** - Negative fixtures exist at `tests/fixtures/stdlib_json/json_parse_trailing_garbage.ark`, `json_parse_trailing_object_garbage.ark`, and `json_parse_multiple_values.ark`. The `parse()` function performs whole-document validation and returns `Err(JsonParseError::TrailingCharacters)` for trailing non-whitespace content. Fixtures compile correctly (phase 4 + phase 6 pass).
2. **TOML bounded subset prominently documented with negative fixtures at boundaries** - `std/toml/mod.ark` documents the bounded subset. Negative fixtures exist for: trailing garbage, table headers, empty values, unclosed strings, and array of tables headers (`toml_parse_invalid_array_of_tables.ark` added in this issue).
3. **Text distinguishes byte vs char operations in doc comments** - `std/text/mod.ark` doc comments explicitly document byte/ASCII orientation of operations, best-effort `len_chars` semantics, and ASCII-only case/trim behavior.
4. **Time / std::host::clock split explicit in docs and API surface** - `std::time` module provides pure duration helpers only (no host clock access). `std::host::clock` owns host clock reads (`monotonic_now`, `now_ms`). The split is documented in module-level doc comments in both modules.

Changes made:

- `tests/fixtures/stdlib_toml/toml_parse_invalid_array_of_tables.ark`: New negative fixture for array-of-tables rejection.
- `tests/fixtures/manifest.txt`: Registered new fixture.

Verification:

- `python3 scripts/manager.py verify quick`: No new failures.
- `python3 scripts/manager.py verify fixtures`: No failing fixtures.
- All JSON negative fixtures compile correctly and reject trailing garbage.
- All TOML negative fixtures compile correctly and reject unsupported subset.
