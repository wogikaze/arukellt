# Stdlib: module family ごとの拡張原則を定義する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 360
**Depends on**: —
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 5

## Summary

core / collections / text / seq / path / json / toml / bytes / csv / test / component / wit / host の各 module family について、expansion target / maintenance only / frozen の区分を定め、API 追加の条件を明文化する。無秩序な関数追加を防ぎ、「Wasm target で実装可能」「diagnostics / docs / tests が揃う」「LLM フレンドリーな命名」の 3 条件を gate にする。

## Current state

- 26 source-backed modules、267 関数が manifest に登録
- カテゴリ分布が偏っている: misc が最大、host family は薄い
- module family ごとの拡張方針が明文化されていない
- `docs/stdlib/stability-policy.md` は stability tier を定義するが、family ごとの拡張原則ではない

## Acceptance

- [ ] 各 module family に expansion / maintenance / frozen のラベルが付与される
- [ ] API 追加時の gate 条件 (fixture / docs / target 互換性) が文書化される
- [ ] `docs/stdlib/` に expansion policy 文書が存在する
- [ ] manifest metadata に family 分類フィールドが追加される

## References

- `std/manifest.toml` — 全 module / function 定義
- `docs/stdlib/stability-policy.md` — stability tier
- `docs/stdlib/README.md` — module 一覧
