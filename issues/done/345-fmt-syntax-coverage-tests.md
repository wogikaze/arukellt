---
Status: done
Created: 2026-03-31
Updated: 2026-03-31
ID: 345
Track: formatter
Depends on: 343
Orchestration class: implementation-ready
Blocks v1 exit: no
Priority: 14
---

# Formatter: 現行構文全域のスナップショットテストを追加する
- `crates/ark-parser/src/fmt.rs: 879-917` — 既存 4 テスト
# Formatter: 現行構文全域のスナップショットテストを追加する

## Summary

formatter のテストを 4 件の最小テストから、現行言語 surface 全域をカバーするスナップショット体系に拡張する。構文要素ごとの golden file テストにより、formatter の変更が既存出力を壊さないことを CI で保証する。

## Current state

- `crates/ark-parser/src/fmt.rs:879-917`: 4 テストのみ (format_simple_function, format_imports_sorted, format_idempotent, format_struct_def)
- enum, trait, impl, match, closure, generic, tuple, array, range, method chain, nested block, complex type annotation のテストなし
- コメント付きコードのテストなし (#343 完了後に追加すべき)

## Acceptance

- [x] enum / struct / trait / impl / match / closure / generic / tuple のフォーマットテストが存在する
- [x] コメント付きコードの idempotency テストが存在する (#343 前提)
- [x] nested expression / method chain / multi-line function call のテストが存在する
- [x] CI で全テストが pass する

## References

- `crates/ark-parser/src/fmt.rs:879-917` — 既存 4 テスト
- `crates/ark-parser/src/ast.rs` — 現行 AST 構文一覧