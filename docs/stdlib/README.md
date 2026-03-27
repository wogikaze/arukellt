# 標準ライブラリ概要

> **Current reality**: 現在の実装状況は [docs/current-state.md](../current-state.md) と `std/*.ark` を基準にしてください。

Arukellt の stdlib は、現在は **compiler intrinsic と source-backed wrapper の併用**です。
公開 API の多くは `std/prelude.ark` から見え、いくつかは `std/io/*.ark` や `std/collections/*.ark` に切り出し始めています。

## 現在使える主な公開面

### Prelude から使えるもの

- 出力: `println`, `print`, `eprintln`
- String: `String_from`, `String_new`, `eq`, `concat`, `clone`, `slice`, `split`, `join`, `starts_with`, `ends_with`, `to_lower`, `to_upper`, `push_char`
- 変換: `i32_to_string`, `i64_to_string`, `f64_to_string`, `bool_to_string`, `char_to_string`, `parse_i32`, `parse_i64`, `parse_f64`
- Math: `sqrt`, `abs`, `min`, `max`, `clamp_i32`
- Vec: `Vec_new_i32`, `Vec_new_i64`, `Vec_new_f64`, `Vec_new_String`, `sort_i32`, `sort_i64`, `sort_f64`, `sort_String`
- Vec HOF: `map_i32_i32`, `filter_i32`, `fold_i32_i32`, `map_i64_i64`, `filter_i64`, `fold_i64_i64`, `map_f64_f64`, `filter_f64`, `map_String_String`, `filter_String`, `any_i32`, `find_i32`
- Helpers: `contains_i32`, `contains_String`, `reverse_i32`, `reverse_String`, `remove_i32`, `sum_i32`, `product_i32`
- Option helper: `map_option_i32_i32`
- I/O: `fs_read_file`, `fs_write_file`, `clock_now`, `random_i32`
- Assert: `assert`, `assert_eq`, `assert_ne`, `assert_eq_i64`, `assert_eq_str`

### Source-backed modules

現在確認できるファイル:

- `std/prelude.ark`
- `std/collections/string.ark`
- `std/io/fs.ark`
- `std/io/clock.ark`
- `std/io/random.ark`

これらは多くの場合、compiler intrinsic への薄い wrapper です。

## まだ設計文書寄りの話

以下は将来の module layout や capability model を含むため、現実装とは一致しない箇所があります。

- `core/*` を source of truth にする構成
- capability-based I/O の将来設計
- Wasm GC 前提の stdlib 型表現

必要なら設計参照として読む価値はありますが、**今書けるコードの基準にはしないでください**。

## 参照先

- [コア API](core.md)
- [I/O API](io.md)
- [Cookbook](cookbook.md)
- [Current state](../current-state.md)
