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

## Acceptance

- [ ] `std` 内の「数値タグで sum type を表現している公開 API」の棚卸しが作成される
- [ ] `enum + match` に移行すべき対象と、ABI/互換性のために当面 `i32` を残す対象が分類される
- [ ] 少なくとも `std::json`, `std::toml`, `std::wit` の migration 方針が個別に文書化される
- [ ] 「新規 stdlib API は raw numeric tag を公開しない」ルールが docs または issue note に固定される

## Primary paths

- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/wit/mod.ark`
- `std/bytes/mod.ark`
- `docs/stdlib/`

## References

- `issues/done/391-stdlib-component-wit-helper-usability.md`
- `issues/open/054-std-wit-component.md`
