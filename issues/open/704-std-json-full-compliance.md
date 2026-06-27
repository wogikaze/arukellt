---
Status: open
Created: 2026-07-14
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

- [ ] `std::json` passes all JSON test suite fixtures (RFC 8259 conformance)
- [ ] Streaming parse utilities (`skip_ws`, `parse_int_at`, `parse_string_at`,
      `decode_escape`, `find_key`, `get_str`, `get_int`, `content_length`)
      are public in `std::json`
- [ ] `json_encode_string` handles all control characters (`\u00XX` for < 0x20)
- [ ] Unicode escape decode (`\uXXXX`) supports surrogate pairs
- [ ] No compiler-internal JSON implementation files remain outside `std::json`
- [ ] Negative fixtures for malformed JSON (unterminated strings, bad escapes)

## Scope

- `std/json/mod.ark` — add streaming utilities, full escape/unescape
- Compiler DAP/LSP/diagnostics — delegate to `std::json`, delete local copies
