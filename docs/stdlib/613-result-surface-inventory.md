# #613 Phase 0 — Stdlib `Result<_, String>` Surface Inventory

**Issue**: [issues/open/613-errhandle-stdlib-result-surface.md](../../issues/open/613-errhandle-stdlib-result-surface.md)
**Phase**: 0 — read-only audit (no API changes in this slice)
**Method**: `rg -n "Result<" std/` over `std/**/*.ark`, cross-referenced with `std/manifest.toml`.

This inventory enumerates every public stdlib API whose declared return type is
`Result<_, String>` (or equivalently a stringly-typed failure channel). It
records the recommended typed-error replacement based on the variants already
defined in [std/core/error.ark](../../std/core/error.ark) (`Error::ParseError`,
`Error::IoError`, `Error::NotFound`, `Error::InvalidArgument`,
`Error::PermissionDenied`, `Error::Timeout`, etc.).

The "priority" column expresses convergence impact for issue #613 acceptance
criterion 2 (introduce typed error enums for the highest-impact APIs in
`std::host::fs` and `std::json`).

---

## Top-3 high-impact APIs to convert first

Per acceptance criterion 2 of #613, the following are flagged **high** priority
and are the explicit targets of the next slice.

### `std::host::fs` (top 3)

1. `std::host::fs::read_to_string(path) -> Result<String, FsError>` — file read failures now map to typed `FsError` variants (`NotFound`, `PermissionDenied`, `Utf8Error`, `IoError`).
2. `std::host::fs::write_string(path, contents) -> Result<(), FsError>` — write failures now map to typed `FsError` variants (`PermissionDenied`, `NotFound`, `IoError`).
3. `std::host::fs::write_bytes(path, bytes) -> Result<(), FsError>` — same typed `FsError` taxonomy as `write_string`.

### `std::json` (top 3)

`std::json` exposes a single `Result`-returning entry point, so it is its own
top-1. The other two slots are reserved for the parallel converters in
`std::toml` and `std::csv` because they share the same parser-error taxonomy
and should be migrated together.

1. `std::json::parse(s) -> Result<JsonValue, JsonParseError>` — now returns typed parse variants (`EmptyInput`, `InvalidLiteral`, `TrailingCharacters`, `UnexpectedCharacter`).
2. `std::toml::toml_parse(s) -> Result<TomlValue, String>` — same `Error::ParseError` taxonomy.
3. `std::csv::csv_parse_with_header(s) -> Result<Vec<String>, String>` — same `Error::ParseError` taxonomy plus row/column index.

---

## Full inventory

| Module | Function signature | Current error type | Recommended typed-error replacement | Priority |
| --- | --- | --- | --- | --- |
| `std::host::fs` | `read_to_string(path: String) -> Result<String, FsError>` | `FsError` | Converted (`NotFound` / `PermissionDenied` / `Utf8Error` / `IoError`) | **high** |
| `std::host::fs` | `write_string(path: String, contents: String) -> Result<(), FsError>` | `FsError` | Converted (`PermissionDenied` / `NotFound` / `IoError`) | **high** |
| `std::host::fs` | `write_bytes(path: String, bytes: Vec<i32>) -> Result<(), FsError>` | `FsError` | Converted (same taxonomy as `write_string`) | **high** |
| `std::json` | `parse(s: String) -> Result<JsonValue, JsonParseError>` | `JsonParseError` | Converted (`EmptyInput` / `InvalidLiteral` / `TrailingCharacters` / `UnexpectedCharacter`) | **high** |
| `std::toml` | `toml_parse(s: String) -> Result<TomlValue, String>` | `String` | `Result<TomlValue, Error>` (`Error::ParseError { kind: "toml", input }`) | med |
| `std::csv` | `csv_parse_with_header(s: String) -> Result<Vec<String>, String>` | `String` | `Result<Vec<String>, Error>` (`Error::ParseError { kind: "csv", input }`) | med |
| `std::fs` | `read_string(path: String) -> Result<String, String>` | `String` | follow `std::host::fs::read_to_string` (delegates) | med |
| `std::fs` | `write_string(path: String, contents: String) -> Result<(), String>` | `String` | follow `std::host::fs::write_string` (delegates) | med |
| `std::host::http` | `request(method: String, url: String, body: String) -> Result<String, String>` | `String` | `Result<String, Error>` (`IoError` / `Timeout` / `PermissionDenied`) | med |
| `std::host::http` | `get(url: String) -> Result<String, String>` | `String` | same as `request` | med |
| `std::host::sockets` | `connect(host: String, port: i32) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError` / `Timeout` / `PermissionDenied`) | med |
| `std::host::udp` | `send(host: String, port: i32, data: String) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError` / `InvalidArgument`) | low |
| `std::io` | `reader_read(r, n) -> Result<Vec<i32>, String>` | `String` | `Result<Vec<i32>, Error>` (`IoError`) | low |
| `std::io` | `reader_read_exact(r, n) -> Result<Vec<i32>, String>` | `String` | `Result<Vec<i32>, Error>` (`IoError` — distinguish short read) | low |
| `std::io` | `reader_read_all(r) -> Result<Vec<i32>, String>` | `String` | `Result<Vec<i32>, Error>` (`IoError`) | low |
| `std::io` | `reader_read_line(r) -> Result<String, String>` | `String` | `Result<String, Error>` (`IoError` / `Utf8Error`) | low |
| `std::io` | `read_stdin_line() -> Result<String, String>` | `String` | `Result<String, Error>` (`IoError` / `Utf8Error`) | low |
| `std::io` | `writer_write(w, data) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError`) | low |
| `std::io` | `writer_write_str(w, s) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError`) | low |
| `std::io` | `write_all(w, data) -> Result<(), String>` | `String` | `Result<(), Error>` (`IoError`) | low |
| `std::io` | `write_string(w, s) -> Result<(), String>` | `String` | `Result<(), Error>` (`IoError`) | low |
| `std::io` | `writer_flush(w) -> Result<(), String>` | `String` | `Result<(), Error>` (`IoError`) | low |
| `std::io` | `flush(w) -> Result<(), String>` | `String` | `Result<(), Error>` (`IoError`) | low |
| `std::io` | `buf_reader_read_line(br) -> Result<String, String>` | `String` | `Result<String, Error>` (`IoError` / `Utf8Error`) | low |
| `std::io` | `buf_writer_write_str(bw, s) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError`) | low |
| `std::io` | `buf_writer_flush(bw) -> Result<(), String>` | `String` | `Result<(), Error>` (`IoError`) | low |
| `std::io` | `copy_bytes(from, to) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError`) | low |
| `std::io` | `copy(r, w) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`IoError`) | low |
| `std::prelude` | `parse_i32(s: String) -> Result<i32, String>` | `String` | `Result<i32, Error>` (`Error::ParseError { kind: "i32", input }`) | med |
| `std::prelude` | `parse_i64(s: String) -> Result<i64, String>` | `String` | `Result<i64, Error>` (`Error::ParseError { kind: "i64", input }`) | med |
| `std::prelude` | `parse_f64(s: String) -> Result<f64, String>` | `String` | `Result<f64, Error>` (`Error::ParseError { kind: "f64", input }`) | med |

Total: **31** public stdlib APIs currently expose `Result<_, String>`.

### Test helper APIs (informational, not in scope for conversion)

These intentionally accept `Result<_, String>` to bridge legacy fixtures and
should be migrated last, after the production APIs above are converted.

| Module | Function signature | Notes |
| --- | --- | --- |
| `std::test` | `expect_ok_i32(r: Result<i32, String>) -> i32` | test helper |
| `std::test` | `expect_err_string(r: Result<i32, String>) -> String` | test helper |

---

## Family coverage notes

- **`std::host::fs`** — 3/3 public `Result`-returning APIs covered (all listed **high**).
- **`std::json`** — 1/1 public `Result`-returning API covered (the only entry point is **high**).
- **`std::toml`** — 1/1 covered.
- **`std::csv`** — 1/1 covered.
- **`std::core::hash`** — N/A: all hash functions (`hash_i32`, `hash_string`,
  `combine`, `hash_combine`) are total and return `i32`; no `Result` surface
  exists today, so the family has no work in #613 scope.
- **`std::io`** — 17 entries, all **low** priority (await typed-error rollout in
  `std::host::fs` first so the IO layer can adopt the same `Error` shape).
- **`std::prelude`** parse helpers — **med** priority (small, self-contained;
  good follow-on after the high-priority slice).

---

## Stability label follow-up (deferred to slice 2)

Per #613 close gate, when typed errors are introduced the corresponding entries
in [std/manifest.toml](../../std/manifest.toml) must have their `stability`
labels reviewed. The manifest currently records `returns = "Result<_, String>"`
in the lines surfaced by `rg -n "Result<.*String>" std/manifest.toml` (~25
occurrences across the families above). No manifest edits are made in this
slice.

---

## How this inventory was produced

```bash
rg -n "Result<" std/
rg -n "Result<.*String>" std/manifest.toml
```

Re-run the same commands to refresh this inventory if new `Result<_, String>`
APIs are added before slice 2 lands.
