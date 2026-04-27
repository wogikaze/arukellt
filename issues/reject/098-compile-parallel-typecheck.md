---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 098
Track: compile-speed
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
Status note: Compiler architecture improvement — deferred to v5+. hello.ark already compiles in 4.2ms.
1. `ark-typecheck/src/checker.rs` の関数チェックに `rayon: ":par_iter` を適用"
# コンパイル速度: "並列型チェック (rayon)"
---
# コンパイル速度: 並列型チェック (rayon)


## Summary

`ark-typecheck` の関数型チェックは関数間の依存がない場合は並列化可能。
`rayon` による並列型チェックを `--opt-level` に依存しない形で導入する。
roadmap-v4.md §10 item 5 で「typecheck の並列化 (rayon) は v4 で評価対象」と明記。

## 受け入れ条件

1. `ark-typecheck/src/checker.rs` の関数チェックに `rayon::par_iter` を適用
2. 依存関係 (型エイリアス・struct 定義) は事前に解決してから並列化
3. `parser.ark` (500行, 多数の関数) で typecheck フェーズが 2コア比 1.5x 以上高速化
4. エラーメッセージの順序が並列化後も確定的であること (sorted by source position)

## 参照

- roadmap-v4.md §10 item 5