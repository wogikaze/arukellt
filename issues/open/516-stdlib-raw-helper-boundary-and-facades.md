# Stdlib: raw helper と推奨 facade の境界を再設計する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 516
**Depends on**: none
**Track**: stdlib
**Blocks v1 exit**: no
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には内部表現に近い helper と user-facing facade が混在しており、
どの API を推奨するかが分かりにくい。raw helper を module-internal または clearly-named low-level tier に寄せ、
推奨 surface は facade 側に集約する。

## Repo evidence

- `std/io/mod.ark` は `Vec<i32>` ベースの reader/writer internal format を公開 surface でも広く使う
- `std/collections/hash_map.ark` / `hash_set.ark` は monomorphic wrapper と low-level helper が混在する
- `std/wit/mod.ark` は interop helper と user-facing meaning helper が混ざりやすい

## Acceptance

- [ ] raw helper / facade / adapter の 3 層分類が family ごとに作られる
- [ ] low-level internal representation を直接公開している API が洗い出される
- [ ] facade 優先の naming policy (`raw_`, `unchecked_`, `internal_` など) が定義される
- [ ] 代表 family (`io`, `collections`, `wit`) の migration sketch が作られる

## Primary paths

- `std/io/mod.ark`
- `std/collections/hash_map.ark`
- `std/collections/hash_set.ark`
- `std/wit/mod.ark`
- `docs/stdlib/`

## References

- `issues/done/384-stdlib-api-admission-gate.md`
