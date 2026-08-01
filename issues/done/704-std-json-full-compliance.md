---
Status: done
Created: 2026-07-14
Updated: 2026-07-26
ID: 704
Track: stdlib
Depends on: 606
---

# std::json Full JSON RFC 8259 Compliance

## Problem

`std/json/mod.ark` currently provides a DOM-level API (`JsonValue` enum,
`parse`, `stringify`) plus legacy primitive helpers. The compiler's DAP/LSP
layers maintain separate streaming JSON parsers (`json_parse_core`,
`json_parse_string`, `json_parse_string_escape`, `json_parse_string_unicode`,
`json_fields`, `json_escape`) that are being consolidated into `std::json`.

## Acceptance criteria

- [x] Representative RFC 8259 +/- fixture corpus under
      `tests/fixtures/stdlib_json/rfc8259/` (plan Phase 3; not a full external
      JSONTestSuite dump)
- [x] Streaming parse utilities (`skip_ws`, `parse_int_at`,
      `json_parse_string_at`, `json_decode_escape`, `find_key_pos`,
      `json_get_str`, `json_get_int`, `parse_content_length`, `quote_string`)
      are public in `std::json` (and registered in `std/manifest.toml`)
- [x] `json_encode_string` handles all control characters (`\u00XX` for < 0x20)
      — covered by `rfc8259/y_encode_control_chars.ark`
- [x] Unicode escape decode (`\uXXXX`) supports surrogate pairs
      — covered by `rfc8259/y_string_surrogate_pair.ark` and streaming twin
- [x] No compiler-internal JSON *parser/escape implementation* files remain
      outside `std::json` (LSP/DAP local `json_*` modules deleted; callers use
      `std::json` directly). Remaining `diagnostics/json.ark` /
      `main/output_json.ark` are DOM *producers* over `std::json::parser`.
- [x] Negative fixtures for malformed JSON (unterminated / bad escapes /
      unescaped controls / trailing commas / unclosed containers)

## Closure note (2026-07-26)

Lane `wave/704-json-full` completed plan Phases 1–3 and the lane verify gate.

Evidence:

- Commit `37a5467d`: std escapes/surrogates + LSP thin-delegate + first rfc8259 set
- Follow-up on same branch: delete LSP/DAP JSON facades; expand rfc8259 to 18 fixtures;
  register streaming APIs in `std/manifest.toml`

Verification:

- `python3 scripts/manager.py selfhost build-compiler` — PASS
- `python3 scripts/manager.py verify lane` — PASS
- rfc8259 hosted smoke — **18/18 PASS**

Residual (non-blocking for this close):

- Full external JSONTestSuite import not vendored
- DOM number parse still depends on host `parse_f64` (pre-existing; number-heavy
  fixtures may trap under some runner/stub combinations)

## Scope

- `std/json.ark` / `std/json/parser.ark` — streaming utilities, full escape/unescape
- Compiler DAP/LSP — delegate to `std::json`, delete local copies
