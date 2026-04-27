---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
ID: 360
Track: stdlib-api
Depends on: —
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 5
---

# Stdlib: module family ごとの拡張原則を定義する
- Added `family = "expansion"` to `std: ":host::http` and `std::host::sockets` in `std/manifest.toml`"
# Stdlib: module family ごとの拡張原則を定義する

## Summary

core / collections / text / seq / path / json / toml / bytes / csv / test / component / wit / host の各 module family について、expansion target / maintenance only / frozen の区分を定め、API 追加の条件を明文化する。

## Acceptance

- [x] 各 module family に expansion / maintenance / frozen のラベルが付与される
- [x] API 追加時の gate 条件 (fixture / docs / target 互換性) が文書化される
- [x] `docs/stdlib/` に expansion policy 文書が存在する
- [x] manifest metadata に family 分類フィールドが追加される

## Resolution

- Created `docs/stdlib/expansion-policy.md` with family classification table (24 families), label definitions, and API addition gate conditions
- Added `family` field to `ManifestModule` in `crates/ark-stdlib/src/lib.rs` with `family_for_module()` method
- Added `family = "expansion"` to `std::host::http` and `std::host::sockets` in `std/manifest.toml`
- Unit test `family_field_parsed` verifies family metadata round-trips through the manifest parser
- 6 ark-stdlib tests passing