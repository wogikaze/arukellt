# Stdlib: allocation / complexity / perf footgun を family 横断で監査する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-18
**ID**: 520
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には repeated `concat`, repeated `slice`, repeated linear scan など、
correctness とは別の performance footgun が残っている。学習用コードとしても実用 surface としても、
allocation と complexity の悪い例を減らし、builder / buffering / better algorithm を優先する方針へ寄せる。

## Repo evidence

- `std/json/mod.ark`, `std/csv/mod.ark`, `std/io/mod.ark`, `std/bytes/mod.ark` には repeated string concat が多い
- parser / formatter 系に repeated `slice` と linear scan が多い
- path / text / bytes family は small helper の組み合わせで O(n^2) になりやすい

## Audit inventory

This slice is grounded in the current source files listed in the work order. The goal is not to fix the code, only to classify the current perf footguns and record the replacement pattern that a follow-up implementation should use.

### `std::json`

| Hotspot | Axis | Evidence | Recommended replacement pattern | Benchmark target |
|---|---|---|---|---|
| `json_get` builds `needle` with nested `concat`, linearly scans the raw object text, and calls `parse(slice(...))` for the matched value | `concat` chains, linear scan, repeated parse | `std/json/mod.ark:293-320` | Replace with a single-pass object scanner that records value spans, then parse the matched span once; use `std::text::builder` only for any string assembly at the edge | Yes |
| `json_get_index` linearly scans the array text and reparses the matched element from a slice | linear scan, repeated parse | `std/json/mod.ark:334-358` | Replace with a span-based array iterator that returns the target element without re-scanning or reparsing sibling elements | Yes |
| `json_decode_string` appends one byte/escape at a time with repeated `concat` and `slice` calls | `concat` chains, needless allocation | `std/json/mod.ark:363-384` | Replace with builder-style accumulation and a single final materialization step; avoid per-character string growth | Yes |
| `json_encode_string` appends one byte/escape at a time with repeated `concat` and `slice` calls | `concat` chains, needless allocation | `std/json/mod.ark:388-402` | Replace with builder-style accumulation or a pre-sized buffer, then emit the final quoted string once | Yes |

Recommended family pattern: parse once, keep spans or cached parsed values for object/array access, and use `std::text::builder` for emission paths instead of nested string growth.

### `std::csv`

| Hotspot | Axis | Evidence | Recommended replacement pattern | Benchmark target |
|---|---|---|---|---|
| `csv_parse_row` grows quoted fields character-by-character with `concat`, and unquoted fields still walk the line byte-by-byte | `concat` chains, linear scan, needless allocation | `std/csv/mod.ark:18-68` | Replace with a single-pass field boundary scanner that records start/end spans, then materialize each field once | Yes |
| `csv_stringify_row` nests `concat` calls for commas, quotes, and escaped content | `concat` chains | `std/csv/mod.ark:74-91` | Replace with `std::text::builder` or a join-style assembly pass that appends each field once | Yes |
| `csv_count_rows`, `csv_get_row_raw`, and `csv_parse_with_header` each call `split(s, "\n")` on the full document | needless allocation, repeated parse | `std/csv/mod.ark:96-152` | Replace with a newline-offset index or one-pass row scan so callers can reuse row spans without re-splitting the document | Yes |

Recommended family pattern: scan the CSV document once, keep row/field spans, and only build strings at the serialization boundary.

### `std::io`

| Hotspot | Axis | Evidence | Recommended replacement pattern | Benchmark target |
|---|---|---|---|---|
| `reader_read_line` and `buf_reader_read_line` append one byte at a time with `__intrinsic_concat` | `concat` chains, needless allocation | `std/io/mod.ark:95-116`, `std/io/mod.ark:294-313` | Replace with buffered byte accumulation and a single conversion to `String` at line end; keep the line as bytes until the boundary | Yes |
| `print_bytes`, `writer_write`, and `buf_writer_flush` rebuild output strings from bytes one byte at a time | `concat` chains, needless allocation | `std/io/mod.ark:153-200`, `std/io/mod.ark:332-385` | Prefer buffered writer paths and convert buffers to text once instead of per-byte concatenation | Yes |
| `reader_from_bytes` and `writer_to_bytes` copy the full buffer into a fresh `Vec<i32>` | needless allocation | `std/io/mod.ark:24-37`, `std/io/mod.ark:253-268` | Replace with ownership-preserving or view-style helpers where possible; otherwise keep these as explicit boundary adapters only | Yes |

Recommended family pattern: keep bytes as bytes through the I/O layer, buffer at the boundary, and only materialize `String` values once per logical record.

### `std::bytes`

| Hotspot | Axis | Evidence | Recommended replacement pattern | Benchmark target |
|---|---|---|---|---|
| `bytes_from_string` walks the source string byte-by-byte and performs an unused one-byte `slice` on every iteration before pushing raw bytes | repeated parse, needless allocation | `std/bytes/mod.ark:24-34` | Replace with a direct byte-view conversion path that pushes raw bytes without materializing per-character slices | Yes |
| `string_from_bytes` rebuilds output with `concat` once per byte | `concat` chains, needless allocation | `std/bytes/mod.ark:37-47` | Replace with builder-style accumulation or buffered emission, then materialize the final `String` once | Yes |
| `bytes_concat` linearly scans both input buffers and copies them into a fresh allocation | linear scan, needless allocation | `std/bytes/mod.ark:80-95` | Replace with pre-sized buffer assembly or a shared extend helper that reserves once and copies once | Yes |
| `bytes_slice` linearly scans the requested range and always allocates a fresh buffer | linear scan, needless allocation | `std/bytes/mod.ark:98-106` | Replace with a span/view-style helper where possible, or keep slicing at a single boundary copy site only | Yes |
| `hex_decode` slices out one-character strings for every nibble and re-parses them through `hex_val_char` | repeated parse, linear scan | `std/bytes/mod.ark:262-278` | Replace with direct nibble decoding over the source string, avoiding per-character slice materialization | Yes |
| `base64_encode` appends every output character with repeated `concat` calls | `concat` chains, needless allocation | `std/bytes/mod.ark:927-961` | Replace with `std::text::builder` or another buffered emission path that appends each 4-byte block once | Yes |
| `base64_decode` slices out each 1-byte chunk before decoding, repeating the parse work for every quartet | repeated parse, linear scan | `std/bytes/mod.ark:964-986` | Replace with direct indexed decoding over the source string so each quartet is scanned once | Yes |

Recommended family pattern: keep byte-oriented helpers in byte space until the boundary, use pre-sized buffers or extend helpers for buffer assembly, and avoid per-character string materialization in decode paths.

### Progress

- `std::bytes` perf-footgun inventory slice completed for concat, linear scan, needless allocation, and repeated parse cases.

### Replacement pattern cross-check

- `std/text/string.ark` already exposes the building blocks that later implementation work should prefer: `split`, `join`, `concat`, `replace`, `lines`, `chars`, `from_utf8`, `to_utf8_bytes`, and `index_of`.
- `std/text/builder.ark` provides the intended accumulation surface for string emission paths, even though the current implementation is still concat-backed.
- The `std::text` helpers are therefore the right replacement family for JSON / CSV string assembly, while buffered I/O is the right replacement family for `std::io` byte-to-string hot paths.

### Benchmark follow-ups

- `std::json`: benchmark `json_get`, `json_get_index`, `json_decode_string`, and `json_encode_string` on nested objects, arrays, and long escaped strings.
- `std::csv`: benchmark `csv_parse_row`, `csv_stringify_row`, `csv_get_row_raw`, and `csv_parse_row_at` on wide rows and multi-row documents.
- `std::io`: benchmark `reader_read_line`, `buf_reader_read_line`, `writer_write`, and `buf_writer_flush` on long lines and large byte payloads.

## Acceptance

- [ ] family ごとの perf footgun inventory が作成される
- [ ] `concat` 連鎖, linear scan, needless allocation, repeated parse の 4 類型で分類される
- [ ] `text::builder`, buffered I/O, pre-sized vec, better search strategy など推奨置換パターンが決まる
- [ ] benchmark へ繋げるべき hotspot が特定される

## Primary paths

- `std/json/mod.ark`
- `std/csv/mod.ark`
- `std/io/mod.ark`
- `std/bytes/mod.ark`
- `std/text/`

## References

- `issues/done/387-stdlib-bytes-buffered-io-helpers.md`
- `issues/done/385-stdlib-text-unicode-conformance.md`
