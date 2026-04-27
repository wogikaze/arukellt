---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 57
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
---
# Wasm GC Abstract Heap Types: any/eq/none/func/nofunc の完全活用
**Blocks v4 exit**: no

**Status note**: Partially implemented. Remaining items deferred to v5+.

## Summary

WasmGC 提案の抽象ヒープ型 (`anyref`, `eqref`, `structref`, `arrayref`, `noneref`, `funcref`, `nofuncref`)
を Arukellt の型システムにマッピングし、最小限の型キャストで済む型階層設計に見直す。
現在 `anyref` へのキャストが多い場合、`eqref` や具体型 `(ref $T)` を使うことで
`br_on_cast` の連鎖を短縮し実行時のキャスト回数を削減できる。

## 受け入れ条件

1. Arukellt の各型と Wasm abstract heap type の明示的マッピング表を `docs/compiler/wasm-type-map.md` に作成
2. `struct.new` / `array.new` の結果型が不必要に `anyref` に昇格しないよう T3 を修正
3. `br_on_cast` の連鎖を `br_on_cast_fail` + 具体型で最短化
4. 型マッピング表に基づいた lint (`--validate-types` フラグ)

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §堆型階層
- `docs/spec/spec-3.0.0/OVERVIEW.md` §GC詳細