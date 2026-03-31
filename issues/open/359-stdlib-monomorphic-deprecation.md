# Stdlib: monomorphic API 群の整理と canonical naming への移行

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 359
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 3

## Summary

`Vec_new_i32`、`filter_i32`、`HashMap_i32_i32_*` のような monomorphic 関数名を整理し、generic API が使える場合は deprecated alias として管理する。歴史的理由で残っている命名を、長期的な stdlib product surface から分離する。

## Current state

- `std/manifest.toml`: monomorphic 名の関数が多数存在 (Vec_new_i32, filter_i32 等)
- generic 対応が進んだ場合、monomorphic wrapper は互換性のみの意味になる
- `docs/stdlib/prelude-migration.md` に歴史的経緯の記録あり
- deprecated 化の仕組み (alias / warning) が未整備

## Acceptance

- [ ] monomorphic API の一覧と、対応する generic API の対応表が文書化される
- [ ] deprecated とすべき monomorphic API に deprecation metadata が付与される
- [ ] `std/manifest.toml` に deprecation / alias 管理フィールドが存在する
- [ ] deprecated API の使用時に W-level diagnostic が出る (#348 lint registry 前提)
- [ ] `docs/stdlib/reference.md` で deprecated API が視覚的に区別される

## References

- `std/manifest.toml` — monomorphic 関数定義
- `docs/stdlib/prelude-migration.md` — 歴史的経緯
- `docs/stdlib/reference.md` — API 一覧表示
