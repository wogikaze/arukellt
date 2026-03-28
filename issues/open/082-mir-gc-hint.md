# MIR: gc_hint パス — 短命オブジェクトのパターン検出

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 082
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: no

## Summary

`crates/ark-mir/src/opt/gc_hint.rs` を実装し、
ループ内で毎回 new → すぐ死ぬ短命オブジェクトのパターンを検出して
wasmtime の GC ヒントカスタムセクションにアノテーションする。
roadmap-v4.md §5.2 item 7 で計画されているパス。

wasmtime が GC ヒントをサポートしない場合は no-op とする設計とし、
サポート開始時に自動的に有効化される。

## 受け入れ条件

1. `passes/gc_hint.rs`: ループ内 `struct.new` + 1回使用 + 脱出なしパターン検出
2. カスタムセクション `metadata.code.gc_hint` に対象アロケーションのオフセットを記録
3. wasmtime が GC ヒントを無視しても実行結果が同一
4. `--opt-level 2` でのみ有効
5. ヒントが有効な場合の GC pause 削減をベンチマークで計測

## 参照

- `docs/process/roadmap-v4.md` §5.2 item 7 および §10 item 2
