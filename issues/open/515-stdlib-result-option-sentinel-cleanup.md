# Stdlib: sentinel 値 / raw String error を Result / Option / Error enum に寄せる

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-18
**ID**: 515
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には `0 - 1` を not-found / invalid sentinel として返す古い API や、
`Result<_, String>` に過度に寄った surface がまだ残る。done issue #392 の naming/convention 整理を
実装 rollout まで進め、`Option` / `Result<_, Error>` / typed enum を優先する surface へ寄せる。

## Repo evidence

- `std/bytes/mod.ark`, `std/text/mod.ark`, `std/path/mod.ark`, `std/seq/mod.ark` に `0 - 1` sentinel が残る
- host / io family には `Result<_, String>` が多い
- `std/core/error.ark` はあるが family 横断で十分使われていない

## Acceptance

- [ ] sentinel 値を返す公開 API の一覧が作成される
- [ ] `Option` / `Result` / typed enum へ移行できる候補と、互換維持のため adapter が要る候補が分類される
- [ ] `String` error を `std::core::Error` へ寄せる migration 方針が family ごとに整理される
- [ ] 新規 stdlib API は sentinel `i32` を返さないルールが明文化される

## Primary paths

- `std/bytes/mod.ark`
- `std/text/mod.ark`
- `std/path/mod.ark`
- `std/seq/mod.ark`
- `std/core/error.ark`

## References

- `issues/done/392-stdlib-error-result-conventions.md`

## Inventory (std::bytes)

Source: `std/bytes/mod.ark` (read-only audit). Classification uses this issue’s axes: prefer **`Option`** when the only failure is absence / EOF, **`Result<_, Error>`** (or a small typed enum) when callers need distinguishable errors, and **`adapter`** for a thin compatibility layer that preserves the old `i32` / `-1` contract next to a modern API.

| Public API | Location | Sentinel / contract | Classification |
|------------|----------|---------------------|----------------|
| `hex_val_char` | `std/bytes/mod.ark:144` (return at `:167`) | Invalid hex digit → `0 - 1` | **Option** (`Option<i32>` nibble) or **Result** (invalid input); **adapter**: keep `-> i32` with documented `-1` beside `try_*` |
| `read_u8` | `:396–404` (`:399`) | No byte remaining → `0 - 1` (doc: “error sentinel”) | **Option** (`None` = EOF); **Result** if cursor errors should be typed; **adapter**: legacy `read_u8` returning `-1` |
| `read_u16_le` | `:409–416` (`:411`) | Fewer than 2 bytes → `0 - 1` | Same family as `read_u8`: **Option** / **Result** for underflow; **adapter** for `-1` preserve |
| `read_u32_le` | `:421–430` (`:423`) | Fewer than 4 bytes → `0 - 1` | Same as `read_u16_le` |
| `read_u32_be` | `:461–470` (`:463`) | Fewer than 4 bytes → `0 - 1` | Same as `read_u32_le` |
| `read_u64_le` | `:435–457` (`:437`) | Fewer than 8 bytes → `i64` `-1` (`i32_to_i64(0) - i32_to_i64(1)`) | Same axis as cursor reads; **Option** / **Result** preferred over numeric sentinel on `i64` |
| `read_bytes` | `:475–487` | Fewer than `n` bytes → **empty** `Vec<i32>` (not `-1`, but same “failed read” idea) | **Option<Vec<i32>>** or **Result**; **adapter**: keep “empty means failure” documented |

**Examples cited (concrete call sites in source):** `hex_val_char` invalid branch at `std/bytes/mod.ark:167`; `read_u8` EOF at `:399`; `read_u32_le` underflow at `:423`.

**Note (non-public helper):** `base64_val` (`std/bytes/mod.ark:500–567`, `:566`) uses the same `0 - 1` invalid pattern; it is not `pub` but shapes `base64_decode` behavior—any future `Option`/`Result` surface for Base64 might lift or replace this helper.

**Negative evidence:** No other `pub fn` in `std/bytes/mod.ark` returns `0 - 1` to callers; `leb128_decode_*` uses `0` or partial decoding for some failure modes (different convention), and `leb128_encode_i32` uses `val == 0 - 1` only as an internal signed-encode state check (`:226`), not as a public return contract.

## Inventory (std::text / std::path / std::seq)

Source-backed audit scope for this slice:

- `std/text/mod.ark:97-150` and `std/text/mod.ark:138-150`
- `std/path/mod.ark:25-183`
- `std/seq/mod.ark:76-95`
- `std/core/error.ark:4-16`

### Family-level migration rule

- `std::text`: return `Option` when the only outcome is absence / no-match, and return `Result<_, Error>` when the call is rejecting invalid input boundaries or other caller-visible misuse. Keep the current `i32` / empty-string forms only as adapter wrappers if a compatibility layer is still required.
- `std::path`: return `Option` for missing components such as file name, extension, or parent; do not encode absence as `""` in the modern surface. Keep any legacy string-returning helpers as thin adapters until the `Path`-typed surface from the path/fs work lands.
- `std::seq`: return `Result` for search operations that need a hit/index vs insertion-point distinction; use a small typed enum only if the family later standardizes on an explicit search-result enum instead of `Result<i32, i32>`. Keep `-1` only in compatibility adapters.
- `std::core::Error` is the shared target for boundary/validation failures; do not introduce new `Result<_, String>` surfaces for these families.

### Inventory (text)

| Public API | Location | Sentinel / contract | Classification |
|------------|----------|---------------------|----------------|
| `slice_bytes` | `std/text/mod.ark:97-104` | Invalid byte boundaries return `""`, which is ambiguous with a valid empty slice | **Result** (`Result<String, Error>`; likely `InvalidArgument` / `IndexOutOfBounds`); **adapter-preserve**: keep the old `String` wrapper only if a compatibility bridge is needed |
| `index_of` | `std/text/mod.ark:138-150` | Not found → `0 - 1` (`-1`) | **Option** (`Option<i32>`); **adapter-preserve**: legacy `i32` sentinel wrapper for the existing surface |

### Inventory (path)

| Public API | Location | Sentinel / contract | Classification |
|------------|----------|---------------------|----------------|
| `file_name` | `std/path/mod.ark:25-33` | Empty path / root path collapses to `""` instead of an explicit absence | **Option** (`Option<String>`) |
| `extension` | `std/path/mod.ark:35-44` | No extension, or leading-dot file treated as "no extension", returns `""` | **Option** (`Option<String>`) |
| `parent` | `std/path/mod.ark:69-77` | Root / no-parent collapses to `""` | **Option** (`Option<String>`) |
| `last_index_of` | `std/path/mod.ark:149-183` | Not found → `0 - 1` (`-1`) | **Option** (`Option<i32>`); if retained temporarily, it can serve as the adapter beneath the higher-level path helpers |

### Inventory (seq)

| Public API | Location | Sentinel / contract | Classification |
|------------|----------|---------------------|----------------|
| `binary_search` | `std/seq/mod.ark:76-95` | Not found → `0 - 1` (`-1`) | **Result** (`Result<i32, i32>` matches the family guidance already recorded in issue #048); **adapter-preserve**: keep `-1` only in a deprecated compatibility wrapper |

**Audit note:** `min_i32`, `max_i32`, `sum_i32`, `count_eq`, `seq_contains`, and the remaining `std/seq` functions do not return a sentinel on their public success path, so they are not part of this migration inventory.
