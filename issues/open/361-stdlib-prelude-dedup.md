# Stdlib: prelude と module import の二重露出を整理する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 361
**Depends on**: 360
**Track**: stdlib-api
**Blocks v1 exit**: no
**Priority**: 7

## Summary

prelude に直結する名前と module import 経由の名前が二重に露出している状態を整理する。どの名前が prelude 直結で、どの名前が `use std::xxx` 経由でのみアクセスすべきかを明確化し、resolver / completion / docs を一貫させる。

## Current state

- `std/prelude.ark` に prelude 関数が定義されている
- 同じ関数が prelude と module の両方からアクセス可能なケースがある
- `crates/ark-resolve/` の resolver は prelude と module import を別経路で解決
- LSP の completion は prelude 名と module 名を区別なく表示

## Acceptance

- [ ] prelude に残す名前と module-only にする名前の基準が文書化される
- [ ] 重複露出する関数の一覧が作成され、各関数の canonical access path が決定される
- [ ] resolver が canonical path を優先して解決する
- [ ] `docs/stdlib/reference.md` が canonical path を表示する

## References

- `std/prelude.ark` — prelude 定義
- `crates/ark-resolve/` — name resolution
- `docs/stdlib/reference.md` — API 一覧
