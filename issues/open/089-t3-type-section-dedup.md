# T3: 関数型セクション重複排除 (Type Section Dedup)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 089
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: no

## Summary

T3 emitter の TypeSection に同一シグネチャの関数型が重複登録されている場合を排除する。
現在 `indirect_types` HashMap が一部重複排除しているが、
GC 型 (struct/array composite types) と関数型のすべてについて
完全な重複排除を保証する実装にする。

## 受け入れ条件

1. 同一 `(param i32 i32) (result i32)` を複数回 TypeSection に追加しない
2. GC struct/array 型の重複排除も確認
3. 型セクションの削減量を計測 (`wasm-objdump --section types`)
4. 既存 fixture への regression なし

## 参照

- roadmap-v4.md §5.3
