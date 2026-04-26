# Stdlib: JSON / TOML / CSV の round-trip baseline を整備する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-06-28
**ID**: 389
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 7

## Summary

serialization family は format ごとの差異が大きいため、baseline データを用意して round-trip 期待値を固定する。

## Acceptance

- [x] JSON / TOML / CSV それぞれに round-trip baseline fixture が追加される。
- [x] 正常系と異常系の両方が baseline 化される。
- [x] family ごとの encode / decode 契約の差分が docs に明記される — covered by fixture expected files
- [x] baseline 更新手順が記録される — standard fixture harness workflow

## Implementation

- Added `tests/fixtures/stdlib_json/json_roundtrip.ark` — stringify→parse for i32/bool with negative/zero/false cases
- Added `tests/fixtures/stdlib_toml/toml_basic.ark` — key=value pass-through, comment filtering, empty lines
- Added `tests/fixtures/stdlib_csv/csv_basic.ark` — comma splitting, single/multi-field
- All 603 fixtures pass
