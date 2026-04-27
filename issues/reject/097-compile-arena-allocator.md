---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 097
Track: compile-speed
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
Status note: Compiler architecture improvement — deferred to v5+. hello.ark already compiles in 4.2ms.
# コンパイル速度: MIR lowering のアリーナ割り当て
---
# コンパイル速度: MIR lowering のアリーナ割り当て


## Summary

`ark-mir` の MIR lowering は多数の `Vec<MirStmt>` / `Box<Operand>` を個別にヒープ確保する。
アリーナアロケータ (bumpalo 等) を導入してフェーズ単位での一括解放を可能にし、
`malloc`/`free` のオーバーヘッドを削減する。

## 受け入れ条件

1. `bumpalo` または `typed-arena` を依存に追加
2. `MirFunction` 内の `Vec<MirStmt>` を arena 内の `&[MirStmt]` に変更
3. `parser.ark` (500行) のコンパイル時間で 10% 以上の改善
4. コンパイラ RSS が arena 導入前より増加しないことを確認

## 参照

- roadmap-v4.md §2 (parser.ark 500ms 目標)