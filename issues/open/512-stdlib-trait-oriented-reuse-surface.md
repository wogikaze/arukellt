# Stdlib: trait ベースの再利用可能 surface へ段階移行する

**Status**: open
**Created**: 2026-04-15
**Updated**: 2026-04-15
**ID**: 512
**Depends on**: 504, 495
**Track**: stdlib
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #495, #504
**Blocks v5**: yes
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

現在の stdlib は concrete type ごとの重複 API や monomorphic wrapper が多く、
再利用性が低い。selfhost trait/impl surface が整い次第、比較・表示・hash・parse などの
共通 protocol を trait 化して family 横断で再利用できる設計へ段階移行する。

## Repo evidence

- `std/collections/hash.ark` 自身が「generic HashMap/HashSet には trait-based hashing が必要」と明記している
- `std/collections/hash_map.ark` / `hash_set.ark` に monomorphic wrapper が多い
- `std/core/error.ark` は enum 化されているが、変換 protocol は family 間で共有されていない

## Acceptance

- [ ] stdlib で trait 化候補となる protocol 一覧 (`Hash`, `Eq`, `Display`, `Parse`, `Writer`, `Reader` など) が作成される
- [ ] trait 導入前でも使える facade 設計案が module family 単位で整理される
- [ ] selfhost trait surface 完了後の migration 順序が決まる
- [ ] monomorphic wrapper と generic target surface の対応表が作られる

## Primary paths

- `std/collections/hash.ark`
- `std/collections/hash_map.ark`
- `std/collections/hash_set.ark`
- `std/core/`
- `docs/adr/`

## References

- `issues/done/359-stdlib-monomorphic-deprecation.md`
- `issues/open/504-selfhost-trait-syntax.md`
- `issues/open/495-selfhost-trait-bounds.md`
