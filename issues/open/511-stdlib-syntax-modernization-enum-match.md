# Stdlib: 数値タグ / if 連鎖を enum + match 優先へ移行する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 511
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には `i32` の数値タグと `else { if ... }` 連鎖で値種別を表現する古い実装がまだ残っている。
読みやすさ・型安全性・将来の拡張性を改善するため、公開 surface と内部モデルの両方で
`enum + match` を優先する方針へ移行する。

## Repo evidence

- `std/json/mod.ark` は `JsonValue { tag: i32, raw: String }` と `v.tag == 0..5` を広く使っている
- `std/toml/mod.ark` も `TomlValue { tag: i32, raw: String }` ベースで型を表現している
- `std/bytes/mod.ark` と `std/wit/mod.ark` に長い `else { if ... }` 連鎖が残っている

## Inventory matrix

対象は「公開 API から見える数値タグ / raw id surface」に限定する。`std::bytes` の `hex_char` / `hex_val_char`
は長い `if` 連鎖を持つが、hex nibble の変換表であり sum-type の public surface ではないため、この acceptance
slice の inventory 対象からは除外する。

| Module / family | Current public surface | Numeric-tag / raw-id exposure | Classification | Reasoning |
|---|---|---|---|---|
| `std::json` value model | `JsonValue { tag: i32, raw: String }`, `parse`, `stringify`, `is_*`, `json_as_*`, `json_get`, `json_get_index` | `tag == 0..5` が公開 struct の判別子として使われる | `migrate to enum+match` | JSON value kind は純粋な sum type であり ABI 固定値ではない。新 surface は `enum JsonValue` を主にし、`match` ベースで判別させるべき。 |
| `std::json` legacy primitive helpers | `json_stringify_i32`, `json_stringify_bool`, `json_stringify_string`, `json_parse_i32`, `json_parse_bool`, `json_null` | 直接の tag 露出はないが、現行 tagged `JsonValue` surface と並走している | `migrate to enum+match` | 互換ラッパーとして当面残してよいが、structured JSON surface の中心は enum 側へ寄せる。legacy helper 自体は raw tag を公開していないため deprecation は別 issue で段階化する。 |
| `std::toml` value model | `TomlValue { tag: i32, raw: String }`, `toml_parse`, `toml_stringify`, `toml_as_*`, `toml_get`, `toml_table_keys` | `tag == 0..5` が公開 struct の判別子として使われる | `migrate to enum+match` | TOML value kind も ABI 上の数値 ID ではなく、公開 tagged struct を維持する理由が弱い。`enum TomlValue` と `match` に移して、extractor/predicate 群は wrapper へ寄せる。 |
| `std::wit` typed constructors / naming | `WitType`, `wit_type_bool` ... `wit_type_string`, `wit_type_name` | 既に enum surface。raw id は直接要求しない | `migrate to enum+match` | この family は公開 surface 側ではほぼ modernized 済み。残課題は `wit_type_from_id` の `else { if ... }` 連鎖を `match` 相当へ置き換えることと、新規 helper を enum-first に保つこと。 |
| `std::wit` id conversion boundary | `wit_type_id(ty)`, `wit_type_from_id(id)` | WIT primitive code を `i32` で往復させる | `keep raw id temporarily for interop/ABI` | WIT / component-model tooling では canonical numeric code との相互変換が必要。raw id surface は削除せず、typed enum との明示的 boundary API に閉じ込める。 |

## Per-module migration notes

### `std::json`

- 目標 surface は `JsonValue` を公開 tagged struct から公開 enum へ移すこと。
- 第一段階では `Null | Bool(bool) | Number(String) | String(String) | Array(String) | Object(String)` のように、再帰制約を避けるため raw text payload を一部 variant payload として残してよい。
- `is_*` / `json_as_*` / `json_get` / `json_get_index` は enum に対する `match` ベース実装へ移し、既存呼び出し側互換のため関数名は当面維持する。
- `JsonValue.tag` を前提にした新 API 追加は禁止し、必要な kind 判定は enum constructor による `match` を前提にする。

### `std::toml`

- 目標 surface は `TomlValue` を公開 tagged struct から公開 enum へ移すこと。
- 現行の `raw: String` 依存は、配列や table を再帰型に即時移行できない間の staging payload として variant payload に押し込める。
- `toml_as_string` / `toml_as_int` / `toml_as_bool` / `toml_get` / `toml_table_keys` は enum を `match` で分岐する wrapper 群として維持できる。
- `toml_parse_line` は legacy helper として別扱いにし、sum-type migration の blocker にはしない。

### `std::wit`

- `WitType` 自体は既に enum なので、migration の中心は「raw id を interop boundary に限定する」こと。
- `wit_type_from_id` の長い `else { if ... }` 連鎖は enum-first surface を壊してはいないが、実装様式としては `match` 優先方針へ揃える対象。
- `wit_type_id` / `wit_type_from_id` は ABI・interop 用 boundary API として当面維持する。
- 新規 WIT helper は `i32` primitive code を直接受け取る設計を避け、`WitType` を主引数にし、必要なら boundary でのみ id 変換する。

## Rule fixed by this issue note

新規 stdlib API は public surface で raw numeric tag を公開しない。
例外は protocol / ABI / interop で canonical numeric code が不可避な場合のみで、その場合も:

- raw code は `*_id` / `*_from_id` のような明示的 conversion boundary に限定する
- 主 API surface は enum / constructor / `match` 前提で設計する
- 「公開 struct の `tag: i32` を読ませる」形は新規採用しない

## Acceptance slice evaluation

- `std::json` / `std::toml` / `std::wit` の公開 surface inventory を本 issue note に記録した
- 各 inventoried family を `migrate to enum+match` または `keep raw id temporarily for interop/ABI` に分類した
- `std::json` / `std::toml` / `std::wit` の module ごとの migration 方針を個別に記録した
- 新規 stdlib API に対する raw numeric tag 非公開ルールを本 issue note に固定した

## Acceptance

- [x] `std` 内の「数値タグで sum type を表現している公開 API」の棚卸しが作成される
- [x] `enum + match` に移行すべき対象と、ABI/互換性のために当面 `i32` を残す対象が分類される
- [x] 少なくとも `std::json`, `std::toml`, `std::wit` の migration 方針が個別に文書化される
- [x] 「新規 stdlib API は raw numeric tag を公開しない」ルールが docs または issue note に固定される

## Primary paths

- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/wit/mod.ark`
- `std/bytes/mod.ark`
- `docs/stdlib/`

## References

- `issues/done/391-stdlib-component-wit-helper-usability.md`
- `issues/open/054-std-wit-component.md`
