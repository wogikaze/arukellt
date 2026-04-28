# Phase 0 Inventory: `Result<_, String>` APIs in Stdlib

> Generated as part of #613 — Error Handling Convergence: Stdlib Result Surface.
> Date: 2026-04-28

## Summary

Total `Result<_, String>` public APIs found across stdlib: **27**

### Already converted to typed errors (0 remaining)

- **`std/host/fs.ark`**: Uses `FsError` (already clean) — 0 remaining
- **`std/json/mod.ark`**: Uses `JsonParseError` (already clean) — 0 remaining

### `std/io/mod.ark` — 16 APIs (highest impact)

All I/O reader/writer/buffer APIs. The only function that actually produces an error is
`reader_read_exact` (`UnexpectedEof`). All others always return `Ok`.

| Line | API | Returns |
|------|-----|---------|
| 47 | `reader_read` | `Result<Vec<i32>, String>` |
| 67 | `reader_read_exact` | `Result<Vec<i32>, String>` |
| 88 | `reader_read_all` | `Result<Vec<i32>, String>` |
| 98 | `reader_read_line` | `Result<String, String>` |
| 148 | `read_stdin_line` | `Result<String, String>` |
| 176 | `writer_write` | `Result<i32, String>` |
| 203 | `writer_write_str` | `Result<i32, String>` |
| 225 | `write_all` | `Result<(), String>` |
| 233 | `write_string` | `Result<(), String>` |
| 244 | `writer_flush` | `Result<(), String>` |
| 249 | `flush` | `Result<(), String>` |
| 296 | `buf_reader_read_line` | `Result<String, String>` |
| 334 | `buf_writer_write_str` | `Result<i32, String>` |
| 353 | `buf_writer_flush` | `Result<(), String>` |
| 388 | `copy_bytes` | `Result<i32, String>` |
| 400 | `copy` | `Result<i32, String>` |

### `std/fs/mod.ark` — 2 APIs

| Line | API | Returns |
|------|-----|---------|
| 25 | `read_string` | `Result<String, String>` |
| 33 | `write_string` | `Result<(), String>` |

### `std/prelude.ark` — 3 APIs (compiler intrinsics)

| Line | API | Returns |
|------|-----|---------|
| 125 | `parse_i32` | `Result<i32, String>` |
| 129 | `parse_i64` | `Result<i64, String>` |
| 133 | `parse_f64` | `Result<f64, String>` |

### `std/host/http.ark` — 2 APIs

| Line | API | Returns |
|------|-----|---------|
| 20 | `request` | `Result<String, String>` |
| 26 | `get` | `Result<String, String>` |

### `std/host/sockets.ark` — 1 API

| Line | API | Returns |
|------|-----|---------|
| 31 | `connect` | `Result<i32, String>` |

### `std/host/udp.ark` — 1 API

| Line | API | Returns |
|------|-----|---------|
| 24 | `send` | `Result<i32, String>` |

### `std/toml/mod.ark` — 1 API

| Line | API | Returns |
|------|-----|---------|
| 114 | `toml_parse` | `Result<TomlValue, String>` |

### `std/csv/mod.ark` — 1 API

| Line | API | Returns |
|------|-----|---------|
| 135 | `csv_parse_with_header` | `Result<Vec<String>, String>` |

## Conversion Priorities

1. **`std/io/mod.ark`** — 16 APIs, highest user impact. `reader_read_exact` is the only
   function that actually produces errors (`UnexpectedEof`).
2. **`std/host/sockets.ark`** / **`std/host/udp.ark`** — network error categories
   (dns, connection, timeout, invalid port) are natural enums.
3. **`std/host/http.ark`** — HTTP error categories.
4. **`std/fs/mod.ark`** — can delegate to `std::host::fs::FsError` directly.
5. **`std/prelude.ark`** — `parse_i32`/etc are compiler intrinsics; need compiler
   work to change return type.
6. **`std/toml/mod.ark`** / **`std/csv/mod.ark`** — lower usage, can follow in
   later issues.
